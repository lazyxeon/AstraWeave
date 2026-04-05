//! Comprehensive mutation-resistant tests for astraweave-cinematics.
//!
//! Targets all 240 mutants across Time, Track, CameraKey, Timeline,
//! Sequencer, SequencerEvent, and their Display/factory/accessor methods.
//! Focuses on: exact return values, cross-variant booleans, match arm
//! deletion coverage, arithmetic boundary values, and format strings.

use astraweave_cinematics::*;

// ============================================================================
// Time — construction, accessors, predicates, arithmetic, Display
// ============================================================================
mod time_tests {
    use super::*;

    // --- Construction ---

    #[test]
    fn from_secs_stores_exact_value() {
        assert_eq!(Time::from_secs(0.0).as_secs(), 0.0);
        assert_eq!(Time::from_secs(1.0).as_secs(), 1.0);
        assert_eq!(Time::from_secs(99.5).as_secs(), 99.5);
        assert_eq!(Time::from_secs(-3.0).as_secs(), -3.0);
    }

    #[test]
    fn from_millis_divides_by_1000() {
        assert_eq!(Time::from_millis(0.0).as_secs(), 0.0);
        assert_eq!(Time::from_millis(1000.0).as_secs(), 1.0);
        assert_eq!(Time::from_millis(500.0).as_secs(), 0.5);
        assert_eq!(Time::from_millis(2500.0).as_secs(), 2.5);
    }

    #[test]
    fn as_millis_multiplies_by_1000() {
        assert_eq!(Time::from_secs(0.0).as_millis(), 0.0);
        assert_eq!(Time::from_secs(1.0).as_millis(), 1000.0);
        assert_eq!(Time::from_secs(0.5).as_millis(), 500.0);
        assert_eq!(Time::from_secs(3.0).as_millis(), 3000.0);
    }

    #[test]
    fn zero_returns_zero_secs() {
        let t = Time::zero();
        assert_eq!(t.as_secs(), 0.0);
        assert!(t.is_zero());
    }

    // --- Predicates ---

    #[test]
    fn is_zero_exact_comparison() {
        assert!(Time(0.0).is_zero());
        assert!(!Time(0.0001).is_zero());
        assert!(!Time(-0.0001).is_zero());
        assert!(!Time(1.0).is_zero());
    }

    #[test]
    fn is_positive_strict_greater_than_zero() {
        assert!(!Time(0.0).is_positive());
        assert!(Time(0.0001).is_positive());
        assert!(Time(1.0).is_positive());
        assert!(!Time(-0.0001).is_positive());
        assert!(!Time(-1.0).is_positive());
    }

    // --- Arithmetic ---

    #[test]
    fn add_secs_returns_sum() {
        let t = Time(2.0);
        assert_eq!(t.add_secs(3.0).as_secs(), 5.0);
        assert_eq!(t.add_secs(0.0).as_secs(), 2.0);
        assert_eq!(t.add_secs(-1.0).as_secs(), 1.0);
    }

    #[test]
    fn clamp_below_returns_min() {
        let t = Time(1.0);
        let clamped = t.clamp(Time(2.0), Time(5.0));
        assert_eq!(clamped.as_secs(), 2.0);
    }

    #[test]
    fn clamp_above_returns_max() {
        let t = Time(10.0);
        let clamped = t.clamp(Time(2.0), Time(5.0));
        assert_eq!(clamped.as_secs(), 5.0);
    }

    #[test]
    fn clamp_within_returns_self() {
        let t = Time(3.0);
        let clamped = t.clamp(Time(2.0), Time(5.0));
        assert_eq!(clamped.as_secs(), 3.0);
    }

    #[test]
    fn clamp_at_min_boundary() {
        let t = Time(2.0);
        let clamped = t.clamp(Time(2.0), Time(5.0));
        assert_eq!(clamped.as_secs(), 2.0);
    }

    #[test]
    fn clamp_at_max_boundary() {
        let t = Time(5.0);
        let clamped = t.clamp(Time(2.0), Time(5.0));
        assert_eq!(clamped.as_secs(), 5.0);
    }

    #[test]
    fn lerp_at_zero_returns_self() {
        let a = Time(2.0);
        let b = Time(8.0);
        assert_eq!(a.lerp(b, 0.0).as_secs(), 2.0);
    }

    #[test]
    fn lerp_at_one_returns_other() {
        let a = Time(2.0);
        let b = Time(8.0);
        assert_eq!(a.lerp(b, 1.0).as_secs(), 8.0);
    }

    #[test]
    fn lerp_at_half_returns_midpoint() {
        let a = Time(0.0);
        let b = Time(10.0);
        assert_eq!(a.lerp(b, 0.5).as_secs(), 5.0);
    }

    #[test]
    fn lerp_at_quarter() {
        let a = Time(0.0);
        let b = Time(100.0);
        assert_eq!(a.lerp(b, 0.25).as_secs(), 25.0);
    }

    // --- Operators ---

    #[test]
    fn add_operator() {
        let r = Time(3.0) + Time(4.0);
        assert_eq!(r.as_secs(), 7.0);
    }

    #[test]
    fn sub_operator() {
        let r = Time(10.0) - Time(3.0);
        assert_eq!(r.as_secs(), 7.0);
    }

    #[test]
    fn add_identity() {
        let r = Time(5.0) + Time(0.0);
        assert_eq!(r.as_secs(), 5.0);
    }

    #[test]
    fn sub_identity() {
        let r = Time(5.0) - Time(0.0);
        assert_eq!(r.as_secs(), 5.0);
    }

    // --- Display ---

    #[test]
    fn display_seconds_branch_gte_one() {
        // >= 1.0 should use "Xs" format
        let s = format!("{}", Time(1.0));
        assert!(s.contains("1.00s"), "got: {}", s);

        let s2 = format!("{}", Time(2.5));
        assert!(s2.contains("2.50s"), "got: {}", s2);

        let s3 = format!("{}", Time(100.0));
        assert!(s3.contains("100.00s"), "got: {}", s3);
    }

    #[test]
    fn display_millis_branch_lt_one() {
        // < 1.0 should use "Xms" format
        let s = format!("{}", Time(0.5));
        assert!(s.contains("500ms"), "got: {}", s);

        let s2 = format!("{}", Time(0.0));
        assert!(s2.contains("0ms"), "got: {}", s2);

        let s3 = format!("{}", Time(0.999));
        assert!(s3.contains("999ms"), "got: {}", s3);
    }

    #[test]
    fn display_boundary_exactly_one() {
        let s = format!("{}", Time(1.0));
        assert!(
            s.contains("s"),
            "should use seconds format at exactly 1.0: {}",
            s
        );
        assert!(!s.contains("ms") || s.contains("1.00s"), "got: {}", s);
    }

    // --- Ordering ---

    #[test]
    fn partial_ord() {
        assert!(Time(1.0) < Time(2.0));
        assert!(Time(2.0) > Time(1.0));
        assert!(Time(1.0) <= Time(1.0));
        assert!(Time(1.0) >= Time(1.0));
    }

    // --- Default ---

    #[test]
    fn default_is_zero() {
        let t = Time::default();
        assert!(t.is_zero());
        assert_eq!(t.as_secs(), 0.0);
    }

    // --- Serde roundtrip ---

    #[test]
    #[allow(clippy::approx_constant)]
    fn time_serde_roundtrip() {
        let t = Time(3.14);
        let json = serde_json::to_string(&t).unwrap();
        let de: Time = serde_json::from_str(&json).unwrap();
        assert_eq!(t, de);
    }
}

// ============================================================================
// Track — variant identification, accessors, factories, Display
// ============================================================================
mod track_tests {
    use super::*;

    // --- type_name: each arm must return its unique string ---

    #[test]
    fn type_name_camera() {
        assert_eq!(Track::camera(vec![]).type_name(), "Camera");
    }

    #[test]
    fn type_name_animation() {
        assert_eq!(
            Track::animation(1, "c", Time::zero()).type_name(),
            "Animation"
        );
    }

    #[test]
    fn type_name_audio() {
        assert_eq!(Track::audio("c", Time::zero(), 1.0).type_name(), "Audio");
    }

    #[test]
    fn type_name_fx() {
        assert_eq!(
            Track::fx("f", Time::zero(), serde_json::json!({})).type_name(),
            "Fx"
        );
    }

    // --- is_camera: true for Camera, false for ALL others ---

    #[test]
    fn is_camera_true() {
        assert!(Track::camera(vec![]).is_camera());
    }

    #[test]
    fn is_camera_false_for_animation() {
        assert!(!Track::animation(1, "c", Time::zero()).is_camera());
    }

    #[test]
    fn is_camera_false_for_audio() {
        assert!(!Track::audio("c", Time::zero(), 1.0).is_camera());
    }

    #[test]
    fn is_camera_false_for_fx() {
        assert!(!Track::fx("f", Time::zero(), serde_json::json!({})).is_camera());
    }

    // --- is_animation: true for Animation, false for ALL others ---

    #[test]
    fn is_animation_true() {
        assert!(Track::animation(1, "c", Time::zero()).is_animation());
    }

    #[test]
    fn is_animation_false_for_camera() {
        assert!(!Track::camera(vec![]).is_animation());
    }

    #[test]
    fn is_animation_false_for_audio() {
        assert!(!Track::audio("c", Time::zero(), 1.0).is_animation());
    }

    #[test]
    fn is_animation_false_for_fx() {
        assert!(!Track::fx("f", Time::zero(), serde_json::json!({})).is_animation());
    }

    // --- is_audio: true for Audio, false for ALL others ---

    #[test]
    fn is_audio_true() {
        assert!(Track::audio("c", Time::zero(), 1.0).is_audio());
    }

    #[test]
    fn is_audio_false_for_camera() {
        assert!(!Track::camera(vec![]).is_audio());
    }

    #[test]
    fn is_audio_false_for_animation() {
        assert!(!Track::animation(1, "c", Time::zero()).is_audio());
    }

    #[test]
    fn is_audio_false_for_fx() {
        assert!(!Track::fx("f", Time::zero(), serde_json::json!({})).is_audio());
    }

    // --- is_fx: true for Fx, false for ALL others ---

    #[test]
    fn is_fx_true() {
        assert!(Track::fx("f", Time::zero(), serde_json::json!({})).is_fx());
    }

    #[test]
    fn is_fx_false_for_camera() {
        assert!(!Track::camera(vec![]).is_fx());
    }

    #[test]
    fn is_fx_false_for_animation() {
        assert!(!Track::animation(1, "c", Time::zero()).is_fx());
    }

    #[test]
    fn is_fx_false_for_audio() {
        assert!(!Track::audio("c", Time::zero(), 1.0).is_fx());
    }

    // --- start_time: None for Camera, Some for others with exact value ---

    #[test]
    fn start_time_camera_is_none() {
        assert!(Track::camera(vec![]).start_time().is_none());
    }

    #[test]
    fn start_time_animation_exact() {
        let t = Track::animation(1, "c", Time(2.5));
        assert_eq!(t.start_time().unwrap().as_secs(), 2.5);
    }

    #[test]
    fn start_time_audio_exact() {
        let t = Track::audio("c", Time(3.0), 0.8);
        assert_eq!(t.start_time().unwrap().as_secs(), 3.0);
    }

    #[test]
    fn start_time_fx_exact() {
        let t = Track::fx("f", Time(4.5), serde_json::json!({}));
        assert_eq!(t.start_time().unwrap().as_secs(), 4.5);
    }

    // --- keyframe_count: Some(n) for Camera, None for others ---

    #[test]
    fn keyframe_count_camera_empty() {
        assert_eq!(Track::camera(vec![]).keyframe_count(), Some(0));
    }

    #[test]
    fn keyframe_count_camera_with_keys() {
        let keys = vec![CameraKey::at_origin(60.0), CameraKey::at_origin(60.0)];
        assert_eq!(Track::camera(keys).keyframe_count(), Some(2));
    }

    #[test]
    fn keyframe_count_animation_none() {
        assert!(Track::animation(1, "c", Time::zero())
            .keyframe_count()
            .is_none());
    }

    #[test]
    fn keyframe_count_audio_none() {
        assert!(Track::audio("c", Time::zero(), 1.0)
            .keyframe_count()
            .is_none());
    }

    #[test]
    fn keyframe_count_fx_none() {
        assert!(Track::fx("f", Time::zero(), serde_json::json!({}))
            .keyframe_count()
            .is_none());
    }

    // --- Factory methods: verify all fields stored correctly ---

    #[test]
    fn factory_camera_stores_keyframes() {
        let keys = vec![CameraKey::new(
            Time(1.0),
            (1.0, 2.0, 3.0),
            (0.0, 0.0, 0.0),
            60.0,
        )];
        let t = Track::camera(keys.clone());
        assert!(t.is_camera());
        assert_eq!(t.keyframe_count(), Some(1));
    }

    #[test]
    fn factory_animation_stores_all_fields() {
        let t = Track::animation(42, "walk", Time(1.5));
        assert!(t.is_animation());
        assert_eq!(t.start_time().unwrap().as_secs(), 1.5);
        // Verify target/clip via Display
        let s = format!("{}", t);
        assert!(s.contains("target=42"), "got: {}", s);
        assert!(s.contains("walk"), "got: {}", s);
    }

    #[test]
    fn factory_audio_stores_all_fields() {
        let t = Track::audio("boom.wav", Time(2.0), 0.75);
        assert!(t.is_audio());
        assert_eq!(t.start_time().unwrap().as_secs(), 2.0);
        let s = format!("{}", t);
        assert!(s.contains("boom.wav"), "got: {}", s);
        assert!(s.contains("0.75"), "got: {}", s);
    }

    #[test]
    fn factory_fx_stores_all_fields() {
        let params = serde_json::json!({"size": 10});
        let t = Track::fx("explode", Time(3.0), params);
        assert!(t.is_fx());
        assert_eq!(t.start_time().unwrap().as_secs(), 3.0);
        let s = format!("{}", t);
        assert!(s.contains("explode"), "got: {}", s);
    }

    // --- Display: verify each variant's format ---

    #[test]
    fn display_camera_shows_keyframe_count() {
        let t = Track::camera(vec![CameraKey::at_origin(60.0)]);
        let s = format!("{}", t);
        assert!(s.contains("Camera"), "got: {}", s);
        assert!(s.contains("1 keyframes"), "got: {}", s);
    }

    #[test]
    fn display_camera_zero_keyframes() {
        let s = format!("{}", Track::camera(vec![]));
        assert!(s.contains("0 keyframes"), "got: {}", s);
    }

    #[test]
    fn display_animation_shows_target_clip_start() {
        let t = Track::animation(99, "run", Time(1.5));
        let s = format!("{}", t);
        assert!(s.contains("Animation"), "got: {}", s);
        assert!(s.contains("target=99"), "got: {}", s);
        assert!(s.contains("run"), "got: {}", s);
        assert!(s.contains("1.50s"), "got: {}", s);
    }

    #[test]
    fn display_audio_shows_clip_start_volume() {
        let t = Track::audio("theme.ogg", Time(0.5), 0.80);
        let s = format!("{}", t);
        assert!(s.contains("Audio"), "got: {}", s);
        assert!(s.contains("theme.ogg"), "got: {}", s);
        assert!(s.contains("500ms"), "got: {}", s);
        assert!(s.contains("0.80"), "got: {}", s);
    }

    #[test]
    fn display_fx_shows_name_start() {
        let t = Track::fx("sparkle", Time(2.0), serde_json::json!({}));
        let s = format!("{}", t);
        assert!(s.contains("Fx"), "got: {}", s);
        assert!(s.contains("sparkle"), "got: {}", s);
        assert!(s.contains("2.00s"), "got: {}", s);
    }

    // --- Serde ---

    #[test]
    fn track_camera_serde_roundtrip() {
        let t = Track::camera(vec![CameraKey::at_origin(60.0)]);
        let json = serde_json::to_string(&t).unwrap();
        let de: Track = serde_json::from_str(&json).unwrap();
        assert_eq!(t, de);
    }

    #[test]
    fn track_animation_serde_roundtrip() {
        let t = Track::animation(10, "dance", Time(1.0));
        let json = serde_json::to_string(&t).unwrap();
        let de: Track = serde_json::from_str(&json).unwrap();
        assert_eq!(t, de);
    }

    #[test]
    fn track_audio_serde_roundtrip() {
        let t = Track::audio("music.ogg", Time(0.0), 0.9);
        let json = serde_json::to_string(&t).unwrap();
        let de: Track = serde_json::from_str(&json).unwrap();
        assert_eq!(t, de);
    }

    #[test]
    fn track_fx_serde_roundtrip() {
        let t = Track::fx("boom", Time(2.0), serde_json::json!({"x": 1}));
        let json = serde_json::to_string(&t).unwrap();
        let de: Track = serde_json::from_str(&json).unwrap();
        assert_eq!(t, de);
    }
}

// ============================================================================
// CameraKey — construction, accessors, math, Display
// ============================================================================
mod camera_key_tests {
    use super::*;

    #[test]
    fn new_stores_all_fields() {
        let k = CameraKey::new(Time(1.0), (2.0, 3.0, 4.0), (5.0, 6.0, 7.0), 90.0);
        assert_eq!(k.t, Time(1.0));
        assert_eq!(k.pos, (2.0, 3.0, 4.0));
        assert_eq!(k.look_at, (5.0, 6.0, 7.0));
        assert_eq!(k.fov_deg, 90.0);
    }

    #[test]
    fn at_origin_sets_correct_defaults() {
        let k = CameraKey::at_origin(75.0);
        assert!(k.t.is_zero());
        assert_eq!(k.pos, (0.0, 0.0, 0.0));
        assert_eq!(k.look_at, (0.0, 0.0, -1.0)); // specific: z = -1
        assert_eq!(k.fov_deg, 75.0);
    }

    #[test]
    fn at_origin_look_at_z_is_negative_one() {
        // Catch mutation: look_at.2 = -1.0 → 1.0 or 0.0
        let k = CameraKey::at_origin(60.0);
        assert_eq!(k.look_at.0, 0.0);
        assert_eq!(k.look_at.1, 0.0);
        assert_eq!(k.look_at.2, -1.0);
    }

    #[test]
    fn position_returns_pos_tuple() {
        let k = CameraKey::new(Time::zero(), (10.0, 20.0, 30.0), (0.0, 0.0, 0.0), 60.0);
        let p = k.position();
        assert_eq!(p.0, 10.0);
        assert_eq!(p.1, 20.0);
        assert_eq!(p.2, 30.0);
    }

    #[test]
    fn distance_to_target_single_axis() {
        // Only X differs
        let k = CameraKey::new(Time::zero(), (5.0, 0.0, 0.0), (0.0, 0.0, 0.0), 60.0);
        assert!((k.distance_to_target() - 5.0).abs() < 0.001);
    }

    #[test]
    fn distance_to_target_3d_pythagorean() {
        // dx=3, dy=4, dz=0 → 5
        let k = CameraKey::new(Time::zero(), (3.0, 4.0, 0.0), (0.0, 0.0, 0.0), 60.0);
        assert!((k.distance_to_target() - 5.0).abs() < 0.001);
    }

    #[test]
    fn distance_to_target_3d_all_axes() {
        // dx=1, dy=2, dz=2 → sqrt(9) = 3
        let k = CameraKey::new(Time::zero(), (1.0, 2.0, 2.0), (0.0, 0.0, 0.0), 60.0);
        assert!((k.distance_to_target() - 3.0).abs() < 0.001);
    }

    #[test]
    fn distance_to_target_zero_when_same() {
        let k = CameraKey::new(Time::zero(), (5.0, 5.0, 5.0), (5.0, 5.0, 5.0), 60.0);
        assert!((k.distance_to_target() - 0.0).abs() < 0.001);
    }

    #[test]
    fn fov_rad_90_degrees() {
        let k = CameraKey::at_origin(90.0);
        assert!((k.fov_rad() - std::f32::consts::FRAC_PI_2).abs() < 0.001);
    }

    #[test]
    fn fov_rad_180_degrees() {
        let k = CameraKey::at_origin(180.0);
        assert!((k.fov_rad() - std::f32::consts::PI).abs() < 0.001);
    }

    #[test]
    fn fov_rad_60_degrees() {
        let k = CameraKey::at_origin(60.0);
        assert!((k.fov_rad() - std::f32::consts::FRAC_PI_3).abs() < 0.001);
    }

    #[test]
    fn is_typical_fov_boundaries() {
        assert!(CameraKey::at_origin(30.0).is_typical_fov()); // lower bound inclusive
        assert!(CameraKey::at_origin(120.0).is_typical_fov()); // upper bound inclusive
        assert!(CameraKey::at_origin(75.0).is_typical_fov()); // middle
        assert!(!CameraKey::at_origin(29.9).is_typical_fov()); // just below
        assert!(!CameraKey::at_origin(120.1).is_typical_fov()); // just above
        assert!(!CameraKey::at_origin(0.0).is_typical_fov());
        assert!(!CameraKey::at_origin(180.0).is_typical_fov());
    }

    // --- Lerp: verify all fields interpolated ---

    #[test]
    fn lerp_at_zero_returns_self() {
        let a = CameraKey::new(Time(0.0), (0.0, 0.0, 0.0), (0.0, 0.0, 0.0), 60.0);
        let b = CameraKey::new(Time(10.0), (10.0, 10.0, 10.0), (10.0, 10.0, 10.0), 120.0);
        let r = a.lerp(&b, 0.0);
        assert_eq!(r.t.as_secs(), 0.0);
        assert_eq!(r.pos, (0.0, 0.0, 0.0));
        assert_eq!(r.look_at, (0.0, 0.0, 0.0));
        assert_eq!(r.fov_deg, 60.0);
    }

    #[test]
    fn lerp_at_one_returns_other() {
        let a = CameraKey::new(Time(0.0), (0.0, 0.0, 0.0), (0.0, 0.0, 0.0), 60.0);
        let b = CameraKey::new(Time(10.0), (10.0, 10.0, 10.0), (10.0, 10.0, 10.0), 120.0);
        let r = a.lerp(&b, 1.0);
        assert_eq!(r.t.as_secs(), 10.0);
        assert_eq!(r.pos, (10.0, 10.0, 10.0));
        assert_eq!(r.look_at, (10.0, 10.0, 10.0));
        assert_eq!(r.fov_deg, 120.0);
    }

    #[test]
    fn lerp_at_half_midpoints() {
        let a = CameraKey::new(Time(0.0), (0.0, 0.0, 0.0), (0.0, 0.0, 0.0), 60.0);
        let b = CameraKey::new(Time(10.0), (10.0, 20.0, 30.0), (4.0, 6.0, 8.0), 120.0);
        let r = a.lerp(&b, 0.5);
        assert_eq!(r.t.as_secs(), 5.0);
        assert_eq!(r.pos.0, 5.0);
        assert_eq!(r.pos.1, 10.0);
        assert_eq!(r.pos.2, 15.0);
        assert_eq!(r.look_at.0, 2.0);
        assert_eq!(r.look_at.1, 3.0);
        assert_eq!(r.look_at.2, 4.0);
        assert_eq!(r.fov_deg, 90.0);
    }

    #[test]
    fn lerp_each_pos_component_independent() {
        // pos.x varies, pos.y and pos.z stay constant
        let a = CameraKey::new(Time(0.0), (0.0, 5.0, 10.0), (0.0, 0.0, 0.0), 60.0);
        let b = CameraKey::new(Time(0.0), (10.0, 5.0, 10.0), (0.0, 0.0, 0.0), 60.0);
        let r = a.lerp(&b, 0.5);
        assert_eq!(r.pos.0, 5.0);
        assert_eq!(r.pos.1, 5.0); // unchanged
        assert_eq!(r.pos.2, 10.0); // unchanged
    }

    #[test]
    fn lerp_each_look_at_component_independent() {
        let a = CameraKey::new(Time(0.0), (0.0, 0.0, 0.0), (0.0, 0.0, 0.0), 60.0);
        let b = CameraKey::new(Time(0.0), (0.0, 0.0, 0.0), (10.0, 20.0, 30.0), 60.0);
        let r = a.lerp(&b, 0.5);
        assert_eq!(r.look_at.0, 5.0);
        assert_eq!(r.look_at.1, 10.0);
        assert_eq!(r.look_at.2, 15.0);
    }

    // --- Display ---

    #[test]
    fn display_shows_time_pos_fov() {
        let k = CameraKey::new(Time(2.5), (1.0, 2.0, 3.0), (0.0, 0.0, 0.0), 90.0);
        let s = format!("{}", k);
        assert!(s.contains("CameraKey"), "got: {}", s);
        assert!(s.contains("2.50s"), "got: {}", s);
        assert!(s.contains("1.0"), "got: {}", s);
        assert!(s.contains("2.0"), "got: {}", s);
        assert!(s.contains("3.0"), "got: {}", s);
        assert!(s.contains("fov=90"), "got: {}", s);
    }

    // --- Serde ---

    #[test]
    fn camera_key_serde_roundtrip() {
        let k = CameraKey::new(Time(1.5), (1.0, 2.0, 3.0), (4.0, 5.0, 6.0), 75.0);
        let json = serde_json::to_string(&k).unwrap();
        let de: CameraKey = serde_json::from_str(&json).unwrap();
        assert_eq!(k, de);
    }
}

// ============================================================================
// Timeline — construction, counting, accessors, Display
// ============================================================================
mod timeline_tests {
    use super::*;

    #[test]
    fn new_stores_name_and_duration() {
        let tl = Timeline::new("intro", 30.0);
        assert_eq!(tl.name, "intro");
        assert_eq!(tl.duration_secs(), 30.0);
        assert!(tl.is_empty());
        assert_eq!(tl.track_count(), 0);
    }

    #[test]
    fn empty_returns_default() {
        let tl = Timeline::empty();
        assert_eq!(tl.name, "");
        assert!(tl.duration.is_zero());
        assert!(tl.is_empty());
    }

    #[test]
    fn default_is_same_as_empty() {
        let d = Timeline::default();
        let e = Timeline::empty();
        assert_eq!(d.name, e.name);
        assert_eq!(d.duration, e.duration);
        assert_eq!(d.tracks.len(), e.tracks.len());
    }

    #[test]
    fn is_empty_with_no_tracks() {
        assert!(Timeline::new("t", 5.0).is_empty());
    }

    #[test]
    fn is_empty_false_with_tracks() {
        let mut tl = Timeline::new("t", 5.0);
        tl.add_track(Track::camera(vec![]));
        assert!(!tl.is_empty());
    }

    #[test]
    fn track_count_increments() {
        let mut tl = Timeline::new("t", 5.0);
        assert_eq!(tl.track_count(), 0);
        tl.add_track(Track::camera(vec![]));
        assert_eq!(tl.track_count(), 1);
        tl.add_track(Track::audio("a", Time::zero(), 1.0));
        assert_eq!(tl.track_count(), 2);
    }

    // --- Per-type counting ---

    #[test]
    fn camera_track_count_only_counts_cameras() {
        let mut tl = Timeline::new("t", 5.0);
        tl.add_track(Track::camera(vec![]));
        tl.add_track(Track::animation(1, "c", Time::zero()));
        tl.add_track(Track::camera(vec![]));
        tl.add_track(Track::audio("a", Time::zero(), 1.0));
        assert_eq!(tl.camera_track_count(), 2);
    }

    #[test]
    fn audio_track_count_only_counts_audio() {
        let mut tl = Timeline::new("t", 5.0);
        tl.add_track(Track::audio("a", Time::zero(), 1.0));
        tl.add_track(Track::camera(vec![]));
        tl.add_track(Track::audio("b", Time::zero(), 0.5));
        assert_eq!(tl.audio_track_count(), 2);
    }

    #[test]
    fn animation_track_count_only_counts_animations() {
        let mut tl = Timeline::new("t", 5.0);
        tl.add_track(Track::animation(1, "a", Time::zero()));
        tl.add_track(Track::audio("x", Time::zero(), 1.0));
        tl.add_track(Track::animation(2, "b", Time::zero()));
        assert_eq!(tl.animation_track_count(), 2);
    }

    #[test]
    fn fx_track_count_only_counts_fx() {
        let mut tl = Timeline::new("t", 5.0);
        tl.add_track(Track::fx("a", Time::zero(), serde_json::json!({})));
        tl.add_track(Track::camera(vec![]));
        tl.add_track(Track::fx("b", Time::zero(), serde_json::json!({})));
        assert_eq!(tl.fx_track_count(), 2);
    }

    #[test]
    fn type_counts_all_zero_when_empty() {
        let tl = Timeline::new("t", 5.0);
        assert_eq!(tl.camera_track_count(), 0);
        assert_eq!(tl.audio_track_count(), 0);
        assert_eq!(tl.animation_track_count(), 0);
        assert_eq!(tl.fx_track_count(), 0);
    }

    // --- add_camera_track / add_audio_track ---

    #[test]
    fn add_camera_track_adds_camera_variant() {
        let mut tl = Timeline::new("t", 5.0);
        tl.add_camera_track(vec![CameraKey::at_origin(60.0)]);
        assert_eq!(tl.camera_track_count(), 1);
        assert_eq!(tl.track_count(), 1);
    }

    #[test]
    fn add_audio_track_adds_audio_variant() {
        let mut tl = Timeline::new("t", 5.0);
        tl.add_audio_track("music", Time(1.0), 0.8);
        assert_eq!(tl.audio_track_count(), 1);
        assert_eq!(tl.track_count(), 1);
    }

    // --- total_keyframes ---

    #[test]
    fn total_keyframes_sums_across_camera_tracks() {
        let mut tl = Timeline::new("t", 5.0);
        tl.add_camera_track(vec![CameraKey::at_origin(60.0), CameraKey::at_origin(60.0)]);
        tl.add_camera_track(vec![CameraKey::at_origin(60.0)]);
        // Non-camera tracks don't contribute
        tl.add_track(Track::animation(1, "c", Time::zero()));
        assert_eq!(tl.total_keyframes(), 3);
    }

    #[test]
    fn total_keyframes_zero_when_no_cameras() {
        let mut tl = Timeline::new("t", 5.0);
        tl.add_track(Track::animation(1, "c", Time::zero()));
        tl.add_track(Track::audio("a", Time::zero(), 1.0));
        assert_eq!(tl.total_keyframes(), 0);
    }

    // --- duration_secs ---

    #[test]
    fn duration_secs_returns_inner_value() {
        let tl = Timeline::new("t", 42.5);
        assert_eq!(tl.duration_secs(), 42.5);
    }

    // --- Display ---

    #[test]
    fn display_shows_name_duration_track_count() {
        let mut tl = Timeline::new("cutscene", 15.0);
        tl.add_track(Track::camera(vec![]));
        tl.add_track(Track::audio("a", Time::zero(), 1.0));
        let s = format!("{}", tl);
        assert!(s.contains("cutscene"), "got: {}", s);
        assert!(s.contains("15.00s"), "got: {}", s);
        assert!(s.contains("2 tracks"), "got: {}", s);
    }

    #[test]
    fn display_empty_timeline() {
        let tl = Timeline::new("empty", 0.0);
        let s = format!("{}", tl);
        assert!(s.contains("empty"), "got: {}", s);
        assert!(s.contains("0 tracks"), "got: {}", s);
    }

    // --- Serde ---

    #[test]
    fn timeline_serde_roundtrip_full() {
        let mut tl = Timeline::new("full", 10.0);
        tl.add_camera_track(vec![CameraKey::at_origin(60.0)]);
        tl.add_track(Track::animation(1, "c", Time(1.0)));
        tl.add_audio_track("music", Time(2.0), 0.8);
        tl.add_track(Track::fx("boom", Time(3.0), serde_json::json!({"x": 1})));
        let json = serde_json::to_string(&tl).unwrap();
        let de: Timeline = serde_json::from_str(&json).unwrap();
        assert_eq!(tl, de);
    }
}

// ============================================================================
// SeqError — Display, thiserror
// ============================================================================
mod seq_error_tests {
    use super::*;

    #[test]
    fn range_error_contains_time_value() {
        let err = SeqError::Range(Time(5.5));
        let msg = format!("{}", err);
        assert!(msg.contains("out of range"), "got: {}", msg);
        assert!(msg.contains("5.5"), "got: {}", msg);
    }

    #[test]
    fn range_error_debug_contains_time() {
        let err = SeqError::Range(Time(3.0));
        let dbg = format!("{:?}", err);
        assert!(dbg.contains("Range"), "got: {}", dbg);
    }
}

// ============================================================================
// Sequencer — seek, step, event emission boundaries
// ============================================================================
mod sequencer_tests {
    use super::*;

    #[test]
    fn new_starts_at_zero() {
        let seq = Sequencer::new();
        assert_eq!(seq.t.as_secs(), 0.0);
    }

    #[test]
    fn default_starts_at_zero() {
        let seq = Sequencer::default();
        assert_eq!(seq.t.as_secs(), 0.0);
    }

    #[test]
    fn seek_sets_time_exactly() {
        let mut seq = Sequencer::new();
        seq.seek(Time(5.0));
        assert_eq!(seq.t.as_secs(), 5.0);
        seq.seek(Time(0.0));
        assert_eq!(seq.t.as_secs(), 0.0);
    }

    #[test]
    fn step_advances_time() {
        let tl = Timeline::new("t", 10.0);
        let mut seq = Sequencer::new();
        seq.step(2.0, &tl).unwrap();
        assert_eq!(seq.t.as_secs(), 2.0);
        seq.step(3.0, &tl).unwrap();
        assert_eq!(seq.t.as_secs(), 5.0);
    }

    #[test]
    fn step_past_duration_errors() {
        let tl = Timeline::new("t", 1.0);
        let mut seq = Sequencer::new();
        let r = seq.step(2.0, &tl);
        assert!(r.is_err());
    }

    #[test]
    fn step_exactly_to_end_ok() {
        let tl = Timeline::new("t", 1.0);
        let mut seq = Sequencer::new();
        // Exactly at duration — within 0.001 tolerance
        assert!(seq.step(1.0, &tl).is_ok());
    }

    #[test]
    fn step_within_tolerance_ok() {
        let tl = Timeline::new("t", 1.0);
        let mut seq = Sequencer::new();
        seq.step(1.0, &tl).unwrap();
        // 0.0005 more — total 1.0005, still within 1.001 tolerance
        assert!(seq.step(0.0005, &tl).is_ok());
    }

    #[test]
    fn step_past_tolerance_errors() {
        let tl = Timeline::new("t", 1.0);
        let mut seq = Sequencer::new();
        seq.step(1.0, &tl).unwrap();
        seq.step(0.0005, &tl).unwrap(); // within tolerance
                                        // Now at ~1.0005, step 0.01 → ~1.0105, > 1.001
        assert!(seq.step(0.01, &tl).is_err());
    }

    // --- Event emission boundary: events fire in (from..=to] ---

    #[test]
    fn event_at_exact_step_boundary_fires() {
        // Event at t=1.0, step from 0 to 1.0 → from=0, to=1.0
        // 1.0 > 0 && 1.0 <= 1.0 → true
        let mut tl = Timeline::new("t", 5.0);
        tl.add_audio_track("hit", Time(1.0), 1.0);
        let mut seq = Sequencer::new();
        let evs = seq.step(1.0, &tl).unwrap();
        assert_eq!(evs.len(), 1);
    }

    #[test]
    fn event_at_from_boundary_does_not_fire() {
        // Event at t=1.0, step from 1.0 to 2.0 → from=1.0, to=2.0
        // 1.0 > 1.0 is false → should NOT fire
        let mut tl = Timeline::new("t", 5.0);
        tl.add_audio_track("hit", Time(1.0), 1.0);
        let mut seq = Sequencer::new();
        seq.step(1.0, &tl).unwrap(); // consumes event
        let evs = seq.step(1.0, &tl).unwrap();
        assert!(evs.is_empty(), "event at from boundary should not re-fire");
    }

    #[test]
    fn event_before_step_range_does_not_fire() {
        // Event at t=0.5, step from 1.0 to 2.0
        let mut tl = Timeline::new("t", 5.0);
        tl.add_audio_track("hit", Time(0.5), 1.0);
        let mut seq = Sequencer::new();
        seq.seek(Time(1.0));
        let evs = seq.step(1.0, &tl).unwrap();
        assert!(evs.is_empty());
    }

    #[test]
    fn event_after_step_range_does_not_fire() {
        // Event at t=3.0, step from 0 to 1.0
        let mut tl = Timeline::new("t", 5.0);
        tl.add_audio_track("hit", Time(3.0), 1.0);
        let mut seq = Sequencer::new();
        let evs = seq.step(1.0, &tl).unwrap();
        assert!(evs.is_empty());
    }

    // --- Each track type emits correct event ---

    #[test]
    fn step_emits_camera_key_event() {
        let mut tl = Timeline::new("t", 5.0);
        tl.add_camera_track(vec![CameraKey::new(
            Time(0.5),
            (1.0, 2.0, 3.0),
            (0.0, 0.0, 0.0),
            60.0,
        )]);
        let mut seq = Sequencer::new();
        let evs = seq.step(1.0, &tl).unwrap();
        assert_eq!(evs.len(), 1);
        assert!(evs[0].is_camera_key());
        let k = evs[0].as_camera_key().unwrap();
        assert_eq!(k.pos, (1.0, 2.0, 3.0));
        assert_eq!(k.fov_deg, 60.0);
    }

    #[test]
    fn step_emits_anim_start_event() {
        let mut tl = Timeline::new("t", 5.0);
        tl.add_track(Track::animation(42, "run", Time(0.5)));
        let mut seq = Sequencer::new();
        let evs = seq.step(1.0, &tl).unwrap();
        assert_eq!(evs.len(), 1);
        assert!(evs[0].is_anim_start());
        assert_eq!(evs[0].animation_clip(), Some("run"));
    }

    #[test]
    fn step_emits_audio_play_event() {
        let mut tl = Timeline::new("t", 5.0);
        tl.add_audio_track("boom", Time(0.5), 0.75);
        let mut seq = Sequencer::new();
        let evs = seq.step(1.0, &tl).unwrap();
        assert_eq!(evs.len(), 1);
        assert!(evs[0].is_audio_play());
        assert_eq!(evs[0].audio_clip(), Some("boom"));
    }

    #[test]
    fn step_emits_fx_trigger_event() {
        let mut tl = Timeline::new("t", 5.0);
        tl.add_track(Track::fx("spark", Time(0.5), serde_json::json!({"s": 1})));
        let mut seq = Sequencer::new();
        let evs = seq.step(1.0, &tl).unwrap();
        assert_eq!(evs.len(), 1);
        assert!(evs[0].is_fx_trigger());
        assert_eq!(evs[0].fx_name(), Some("spark"));
    }

    // --- Multiple events in one step ---

    #[test]
    fn step_emits_all_matching_events() {
        let mut tl = Timeline::new("t", 5.0);
        tl.add_camera_track(vec![CameraKey::at_origin(60.0)]); // t=0.0 (won't fire: 0>0 false)
        tl.add_audio_track("a", Time(0.5), 1.0);
        tl.add_track(Track::animation(1, "w", Time(0.8)));
        tl.add_track(Track::fx("f", Time(0.3), serde_json::json!({})));
        let mut seq = Sequencer::new();
        let evs = seq.step(1.0, &tl).unwrap();
        // audio at 0.5, anim at 0.8, fx at 0.3 → 3 events
        assert_eq!(evs.len(), 3);
    }

    // --- No events in empty timeline ---

    #[test]
    fn step_empty_timeline_no_events() {
        let tl = Timeline::new("t", 5.0);
        let mut seq = Sequencer::new();
        let evs = seq.step(1.0, &tl).unwrap();
        assert!(evs.is_empty());
    }

    // --- Multiple camera keyframes in one track ---

    #[test]
    fn step_multiple_camera_keyframes() {
        let mut tl = Timeline::new("t", 5.0);
        tl.add_camera_track(vec![
            CameraKey::new(Time(0.3), (1.0, 0.0, 0.0), (0.0, 0.0, 0.0), 60.0),
            CameraKey::new(Time(0.7), (2.0, 0.0, 0.0), (0.0, 0.0, 0.0), 60.0),
            CameraKey::new(Time(1.5), (3.0, 0.0, 0.0), (0.0, 0.0, 0.0), 60.0),
        ]);
        let mut seq = Sequencer::new();
        let evs = seq.step(1.0, &tl).unwrap();
        // 0.3 and 0.7 are in (0..=1.0], 1.5 is not
        assert_eq!(evs.len(), 2);
        assert_eq!(evs[0].as_camera_key().unwrap().pos.0, 1.0);
        assert_eq!(evs[1].as_camera_key().unwrap().pos.0, 2.0);
    }
}

// ============================================================================
// SequencerEvent — type_name, is_*, as_*, clip/name, factories, Display
// ============================================================================
mod sequencer_event_tests {
    use super::*;

    fn make_camera_event() -> SequencerEvent {
        SequencerEvent::CameraKey(CameraKey::at_origin(60.0))
    }
    fn make_anim_event() -> SequencerEvent {
        SequencerEvent::AnimStart {
            target: 5,
            clip: "walk".into(),
        }
    }
    fn make_audio_event() -> SequencerEvent {
        SequencerEvent::AudioPlay {
            clip: "music".into(),
            volume: 0.8,
        }
    }
    fn make_fx_event() -> SequencerEvent {
        SequencerEvent::FxTrigger {
            name: "boom".into(),
            params: serde_json::json!({"x": 1}),
        }
    }

    // --- type_name: each variant returns unique string ---

    #[test]
    fn type_name_camera_key() {
        assert_eq!(make_camera_event().type_name(), "CameraKey");
    }

    #[test]
    fn type_name_anim_start() {
        assert_eq!(make_anim_event().type_name(), "AnimStart");
    }

    #[test]
    fn type_name_audio_play() {
        assert_eq!(make_audio_event().type_name(), "AudioPlay");
    }

    #[test]
    fn type_name_fx_trigger() {
        assert_eq!(make_fx_event().type_name(), "FxTrigger");
    }

    // --- is_camera_key: true only for CameraKey ---

    #[test]
    fn is_camera_key_true() {
        assert!(make_camera_event().is_camera_key());
    }

    #[test]
    fn is_camera_key_false_for_anim() {
        assert!(!make_anim_event().is_camera_key());
    }

    #[test]
    fn is_camera_key_false_for_audio() {
        assert!(!make_audio_event().is_camera_key());
    }

    #[test]
    fn is_camera_key_false_for_fx() {
        assert!(!make_fx_event().is_camera_key());
    }

    // --- is_anim_start: true only for AnimStart ---

    #[test]
    fn is_anim_start_true() {
        assert!(make_anim_event().is_anim_start());
    }

    #[test]
    fn is_anim_start_false_for_camera() {
        assert!(!make_camera_event().is_anim_start());
    }

    #[test]
    fn is_anim_start_false_for_audio() {
        assert!(!make_audio_event().is_anim_start());
    }

    #[test]
    fn is_anim_start_false_for_fx() {
        assert!(!make_fx_event().is_anim_start());
    }

    // --- is_audio_play: true only for AudioPlay ---

    #[test]
    fn is_audio_play_true() {
        assert!(make_audio_event().is_audio_play());
    }

    #[test]
    fn is_audio_play_false_for_camera() {
        assert!(!make_camera_event().is_audio_play());
    }

    #[test]
    fn is_audio_play_false_for_anim() {
        assert!(!make_anim_event().is_audio_play());
    }

    #[test]
    fn is_audio_play_false_for_fx() {
        assert!(!make_fx_event().is_audio_play());
    }

    // --- is_fx_trigger: true only for FxTrigger ---

    #[test]
    fn is_fx_trigger_true() {
        assert!(make_fx_event().is_fx_trigger());
    }

    #[test]
    fn is_fx_trigger_false_for_camera() {
        assert!(!make_camera_event().is_fx_trigger());
    }

    #[test]
    fn is_fx_trigger_false_for_anim() {
        assert!(!make_anim_event().is_fx_trigger());
    }

    #[test]
    fn is_fx_trigger_false_for_audio() {
        assert!(!make_audio_event().is_fx_trigger());
    }

    // --- as_camera_key: Some for CameraKey, None for others ---

    #[test]
    fn as_camera_key_some() {
        let ev = make_camera_event();
        assert!(ev.as_camera_key().is_some());
        assert_eq!(ev.as_camera_key().unwrap().fov_deg, 60.0);
    }

    #[test]
    fn as_camera_key_none_for_anim() {
        assert!(make_anim_event().as_camera_key().is_none());
    }

    #[test]
    fn as_camera_key_none_for_audio() {
        assert!(make_audio_event().as_camera_key().is_none());
    }

    #[test]
    fn as_camera_key_none_for_fx() {
        assert!(make_fx_event().as_camera_key().is_none());
    }

    // --- animation_clip: Some for AnimStart, None for others ---

    #[test]
    fn animation_clip_some() {
        assert_eq!(make_anim_event().animation_clip(), Some("walk"));
    }

    #[test]
    fn animation_clip_none_for_camera() {
        assert!(make_camera_event().animation_clip().is_none());
    }

    #[test]
    fn animation_clip_none_for_audio() {
        assert!(make_audio_event().animation_clip().is_none());
    }

    #[test]
    fn animation_clip_none_for_fx() {
        assert!(make_fx_event().animation_clip().is_none());
    }

    // --- audio_clip: Some for AudioPlay, None for others ---

    #[test]
    fn audio_clip_some() {
        assert_eq!(make_audio_event().audio_clip(), Some("music"));
    }

    #[test]
    fn audio_clip_none_for_camera() {
        assert!(make_camera_event().audio_clip().is_none());
    }

    #[test]
    fn audio_clip_none_for_anim() {
        assert!(make_anim_event().audio_clip().is_none());
    }

    #[test]
    fn audio_clip_none_for_fx() {
        assert!(make_fx_event().audio_clip().is_none());
    }

    // --- fx_name: Some for FxTrigger, None for others ---

    #[test]
    fn fx_name_some() {
        assert_eq!(make_fx_event().fx_name(), Some("boom"));
    }

    #[test]
    fn fx_name_none_for_camera() {
        assert!(make_camera_event().fx_name().is_none());
    }

    #[test]
    fn fx_name_none_for_anim() {
        assert!(make_anim_event().fx_name().is_none());
    }

    #[test]
    fn fx_name_none_for_audio() {
        assert!(make_audio_event().fx_name().is_none());
    }

    // --- Factory methods ---

    #[test]
    fn factory_camera_key() {
        let k = CameraKey::new(Time(1.0), (1.0, 2.0, 3.0), (0.0, 0.0, 0.0), 90.0);
        let ev = SequencerEvent::camera_key(k.clone());
        assert!(ev.is_camera_key());
        let inner = ev.as_camera_key().unwrap();
        assert_eq!(inner.pos, (1.0, 2.0, 3.0));
        assert_eq!(inner.fov_deg, 90.0);
    }

    #[test]
    fn factory_anim_start() {
        let ev = SequencerEvent::anim_start(99, "dance");
        assert!(ev.is_anim_start());
        assert_eq!(ev.animation_clip(), Some("dance"));
        // Verify target via Display
        let s = format!("{}", ev);
        assert!(s.contains("target=99"), "got: {}", s);
    }

    #[test]
    fn factory_audio_play() {
        let ev = SequencerEvent::audio_play("track.mp3", 0.6);
        assert!(ev.is_audio_play());
        assert_eq!(ev.audio_clip(), Some("track.mp3"));
        let s = format!("{}", ev);
        assert!(s.contains("0.60"), "got: {}", s);
    }

    #[test]
    fn factory_fx_trigger() {
        let ev = SequencerEvent::fx_trigger("sparkle", serde_json::json!({"count": 50}));
        assert!(ev.is_fx_trigger());
        assert_eq!(ev.fx_name(), Some("sparkle"));
    }

    // --- Display: each variant format ---

    #[test]
    fn display_camera_key_event() {
        let ev = SequencerEvent::camera_key(CameraKey::new(
            Time(1.5),
            (1.0, 2.0, 3.0),
            (0.0, 0.0, 0.0),
            60.0,
        ));
        let s = format!("{}", ev);
        assert!(s.contains("CameraKey"), "got: {}", s);
    }

    #[test]
    fn display_anim_start_event() {
        let ev = SequencerEvent::anim_start(10, "attack");
        let s = format!("{}", ev);
        assert!(s.contains("AnimStart"), "got: {}", s);
        assert!(s.contains("target=10"), "got: {}", s);
        assert!(s.contains("attack"), "got: {}", s);
    }

    #[test]
    fn display_audio_play_event() {
        let ev = SequencerEvent::audio_play("bgm.ogg", 0.90);
        let s = format!("{}", ev);
        assert!(s.contains("AudioPlay"), "got: {}", s);
        assert!(s.contains("bgm.ogg"), "got: {}", s);
        assert!(s.contains("0.90"), "got: {}", s);
    }

    #[test]
    fn display_fx_trigger_event() {
        let ev = SequencerEvent::fx_trigger("fire", serde_json::json!({}));
        let s = format!("{}", ev);
        assert!(s.contains("FxTrigger"), "got: {}", s);
        assert!(s.contains("fire"), "got: {}", s);
    }

    // --- Serde ---

    #[test]
    fn sequencer_event_serde_roundtrip_camera() {
        let ev = SequencerEvent::camera_key(CameraKey::at_origin(60.0));
        let json = serde_json::to_string(&ev).unwrap();
        let de: SequencerEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(ev, de);
    }

    #[test]
    fn sequencer_event_serde_roundtrip_anim() {
        let ev = SequencerEvent::anim_start(1, "walk");
        let json = serde_json::to_string(&ev).unwrap();
        let de: SequencerEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(ev, de);
    }

    #[test]
    fn sequencer_event_serde_roundtrip_audio() {
        let ev = SequencerEvent::audio_play("music", 0.5);
        let json = serde_json::to_string(&ev).unwrap();
        let de: SequencerEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(ev, de);
    }

    #[test]
    fn sequencer_event_serde_roundtrip_fx() {
        let ev = SequencerEvent::fx_trigger("boom", serde_json::json!({"size": 5}));
        let json = serde_json::to_string(&ev).unwrap();
        let de: SequencerEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(ev, de);
    }

    // --- Equality ---

    #[test]
    fn event_equality_same() {
        let a = SequencerEvent::audio_play("a", 1.0);
        let b = SequencerEvent::audio_play("a", 1.0);
        assert_eq!(a, b);
    }

    #[test]
    fn event_inequality_different_clip() {
        let a = SequencerEvent::audio_play("a", 1.0);
        let b = SequencerEvent::audio_play("b", 1.0);
        assert_ne!(a, b);
    }

    #[test]
    fn event_inequality_different_volume() {
        let a = SequencerEvent::audio_play("a", 1.0);
        let b = SequencerEvent::audio_play("a", 0.5);
        assert_ne!(a, b);
    }

    #[test]
    fn event_inequality_different_variant() {
        let a = SequencerEvent::audio_play("a", 1.0);
        let b = SequencerEvent::anim_start(1, "a");
        assert_ne!(a, b);
    }
}

// ============================================================================
// Integration — full timeline playback scenarios
// ============================================================================
mod integration_tests {
    use super::*;

    #[test]
    fn full_cutscene_playback() {
        let mut tl = Timeline::new("boss_intro", 5.0);
        tl.add_camera_track(vec![
            CameraKey::new(Time(0.5), (0.0, 5.0, 10.0), (0.0, 0.0, 0.0), 60.0),
            CameraKey::new(Time(2.0), (5.0, 5.0, 5.0), (0.0, 0.0, 0.0), 75.0),
        ]);
        tl.add_audio_track("boss_theme", Time(0.1), 0.9);
        tl.add_track(Track::animation(1, "boss_enter", Time(1.0)));
        tl.add_track(Track::fx(
            "lightning",
            Time(3.0),
            serde_json::json!({"bolts": 3}),
        ));

        assert_eq!(tl.track_count(), 4);
        assert_eq!(tl.camera_track_count(), 1);
        assert_eq!(tl.audio_track_count(), 1);
        assert_eq!(tl.animation_track_count(), 1);
        assert_eq!(tl.fx_track_count(), 1);
        assert_eq!(tl.total_keyframes(), 2);

        let mut seq = Sequencer::new();

        // Step 0→1: audio at 0.1, camera at 0.5, anim at 1.0 (1.0>0 && 1.0<=1.0)
        let evs = seq.step(1.0, &tl).unwrap();
        assert_eq!(evs.len(), 3);

        // Step 1→2: camera at 2.0 (2.0>1.0 && 2.0<=2.0)
        // anim at 1.0: 1.0 > 1.0 → false, already consumed
        let evs = seq.step(1.0, &tl).unwrap();
        assert_eq!(evs.len(), 1);
        assert!(evs[0].is_camera_key());

        // Step 2→3.5: fx at 3.0
        let evs = seq.step(1.5, &tl).unwrap();
        assert_eq!(evs.len(), 1);
        assert!(evs[0].is_fx_trigger());

        // Step 3.5→5.0: no more events
        let evs = seq.step(1.5, &tl).unwrap();
        assert!(evs.is_empty());
    }

    #[test]
    fn rapid_small_steps_catch_all_events() {
        let mut tl = Timeline::new("t", 2.0);
        tl.add_audio_track("a", Time(0.1), 1.0);
        tl.add_audio_track("b", Time(0.2), 1.0);
        tl.add_audio_track("c", Time(0.3), 1.0);

        let mut seq = Sequencer::new();
        let mut total_events = 0;
        for _ in 0..20 {
            let evs = seq.step(0.1, &tl).unwrap();
            total_events += evs.len();
        }
        assert_eq!(total_events, 3);
    }

    #[test]
    fn seek_then_step_skips_earlier_events() {
        let mut tl = Timeline::new("t", 5.0);
        tl.add_audio_track("early", Time(0.5), 1.0);
        tl.add_audio_track("late", Time(3.0), 1.0);

        let mut seq = Sequencer::new();
        seq.seek(Time(2.0));
        let evs = seq.step(2.0, &tl).unwrap();
        // Only "late" at 3.0 should fire (3.0 > 2.0 && 3.0 <= 4.0)
        assert_eq!(evs.len(), 1);
        assert_eq!(evs[0].audio_clip(), Some("late"));
    }

    #[test]
    fn timeline_serialization_preserves_all_events_on_playback() {
        let mut tl = Timeline::new("ser_test", 3.0);
        tl.add_audio_track("a", Time(1.0), 0.5);
        tl.add_track(Track::animation(1, "b", Time(2.0)));

        let json = serde_json::to_string(&tl).unwrap();
        let tl2: Timeline = serde_json::from_str(&json).unwrap();

        let mut seq = Sequencer::new();
        let evs = seq.step(3.0, &tl2).unwrap();
        assert_eq!(evs.len(), 2);
    }
}

// ============================================================================
// Edge cases — unusual inputs, zero-length, negative time
// ============================================================================
mod edge_case_tests {
    use super::*;

    #[test]
    fn time_negative_values() {
        let t = Time(-5.0);
        assert!(!t.is_zero());
        assert!(!t.is_positive());
        assert_eq!(t.as_secs(), -5.0);
        assert_eq!(t.as_millis(), -5000.0);
    }

    #[test]
    fn time_lerp_extrapolation() {
        // t > 1.0 extrapolates
        let a = Time(0.0);
        let b = Time(10.0);
        let r = a.lerp(b, 2.0);
        assert_eq!(r.as_secs(), 20.0);
    }

    #[test]
    fn camera_key_lerp_same_key() {
        let k = CameraKey::new(Time(1.0), (5.0, 5.0, 5.0), (0.0, 0.0, 0.0), 60.0);
        let r = k.lerp(&k, 0.5);
        assert_eq!(r.t.as_secs(), 1.0);
        assert_eq!(r.pos, (5.0, 5.0, 5.0));
        assert_eq!(r.fov_deg, 60.0);
    }

    #[test]
    fn camera_key_distance_very_large() {
        let k = CameraKey::new(
            Time::zero(),
            (1000.0, 1000.0, 1000.0),
            (0.0, 0.0, 0.0),
            60.0,
        );
        let expected = (3_000_000.0_f32).sqrt();
        assert!((k.distance_to_target() - expected).abs() < 1.0);
    }

    #[test]
    fn timeline_many_tracks() {
        let mut tl = Timeline::new("big", 100.0);
        for i in 0..100 {
            tl.add_audio_track(format!("clip_{}", i), Time::from_secs(i as f32), 1.0);
        }
        assert_eq!(tl.track_count(), 100);
        assert_eq!(tl.audio_track_count(), 100);
        assert_eq!(tl.camera_track_count(), 0);
    }

    #[test]
    fn sequencer_zero_dt_no_events() {
        let mut tl = Timeline::new("t", 5.0);
        tl.add_audio_track("a", Time(1.0), 1.0);
        let mut seq = Sequencer::new();
        // dt=0 → from=0, to=0. Event at 1.0 not in (0..=0]
        let evs = seq.step(0.0, &tl).unwrap();
        assert!(evs.is_empty());
    }

    #[test]
    fn time_clamp_min_equals_max() {
        let t = Time(5.0);
        let clamped = t.clamp(Time(3.0), Time(3.0));
        assert_eq!(clamped.as_secs(), 3.0);
    }

    #[test]
    fn camera_key_zero_fov() {
        let k = CameraKey::at_origin(0.0);
        assert_eq!(k.fov_deg, 0.0);
        assert_eq!(k.fov_rad(), 0.0);
        assert!(!k.is_typical_fov());
    }

    #[test]
    fn track_fx_start_time() {
        let t = Track::fx("x", Time(7.77), serde_json::json!(null));
        assert_eq!(t.start_time().unwrap().as_secs(), 7.77);
    }

    #[test]
    fn sequencer_event_fx_params_preserved() {
        let params = serde_json::json!({"a": 1, "b": [2, 3]});
        let ev = SequencerEvent::fx_trigger("test", params.clone());
        if let SequencerEvent::FxTrigger { params: p, .. } = &ev {
            assert_eq!(*p, params);
        } else {
            panic!("expected FxTrigger");
        }
    }

    #[test]
    fn sequencer_event_anim_target_preserved() {
        let ev = SequencerEvent::anim_start(u32::MAX, "clip");
        if let SequencerEvent::AnimStart { target, clip } = &ev {
            assert_eq!(*target, u32::MAX);
            assert_eq!(clip, "clip");
        } else {
            panic!("expected AnimStart");
        }
    }
}

// ============================================================================
// Mutation Kill Tests — targeted at specific surviving mutations
// ============================================================================
mod mutation_kill_tests {
    use super::*;

    /// Kills `lib.rs:300` `- → +` in CameraKey::lerp pos.0
    /// and `lib.rs:301` `- → +` in CameraKey::lerp pos.1
    ///
    /// With self.pos = (2.0, 3.0, 0.0) and other.pos = (8.0, 9.0, 0.0), t=0.5:
    ///   Original: 2 + (8-2)*0.5 = 5.0, 3 + (9-3)*0.5 = 6.0
    ///   Mutant:   2 + (8+2)*0.5 = 7.0, 3 + (9+3)*0.5 = 9.0  ← DIFFERENT
    #[test]
    fn lerp_pos_with_nonzero_start_discriminates_sub_vs_add() {
        let a = CameraKey::new(Time(0.0), (2.0, 3.0, 0.0), (0.0, 0.0, 0.0), 60.0);
        let b = CameraKey::new(Time(10.0), (8.0, 9.0, 0.0), (0.0, 0.0, 0.0), 60.0);
        let r = a.lerp(&b, 0.5);
        assert_eq!(r.pos.0, 5.0, "pos.0 lerp must subtract, not add");
        assert_eq!(r.pos.1, 6.0, "pos.1 lerp must subtract, not add");
    }

    /// Kills `lib.rs:443` `> → >=` in Sequencer::step (Camera track)
    ///
    /// Events use half-open interval (from, to]. After stepping TO an event's
    /// time, the next step must NOT re-emit the same event (from == event.t).
    #[test]
    fn sequencer_camera_no_duplicate_at_boundary() {
        let mut tl = Timeline::new("test", 2.0);
        tl.add_camera_track(vec![CameraKey::new(
            Time(0.5),
            (0.0, 0.0, 0.0),
            (0.0, 0.0, -1.0),
            60.0,
        )]);
        let mut seq = Sequencer::new();
        // Step from 0.0 to 0.5 — event at 0.5 is in (0.0, 0.5], should emit
        let evs1 = seq.step(0.5, &tl).unwrap();
        assert_eq!(evs1.len(), 1, "first step must emit camera event");
        // Step from 0.5 to 1.0 — event at 0.5 is NOT in (0.5, 1.0], must not re-emit
        let evs2 = seq.step(0.5, &tl).unwrap();
        assert_eq!(evs2.len(), 0, "second step must not duplicate camera event");
    }

    /// Kills `lib.rs:489` `> → >=` in Sequencer::step (Audio track)
    ///
    /// Same boundary logic as camera but for audio tracks.
    #[test]
    fn sequencer_audio_no_duplicate_at_boundary() {
        let mut tl = Timeline::new("test", 2.0);
        tl.add_audio_track("sfx.ogg", Time(0.5), 1.0);
        let mut seq = Sequencer::new();
        // Step from 0.0 to 0.5 — audio at 0.5 is in (0.0, 0.5], should emit
        let evs1 = seq.step(0.5, &tl).unwrap();
        assert_eq!(evs1.len(), 1, "first step must emit audio event");
        // Step from 0.5 to 1.0 — audio at 0.5 is NOT in (0.5, 1.0], must not re-emit
        let evs2 = seq.step(0.5, &tl).unwrap();
        assert_eq!(evs2.len(), 0, "second step must not duplicate audio event");
    }
}
