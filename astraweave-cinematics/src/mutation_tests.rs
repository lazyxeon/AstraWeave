//! Mutation-resistant tests for cinematic systems.
//!
//! These tests are designed to catch common mutations in timeline,
//! sequencer, and camera keyframe logic.

use crate::{CameraKey, Sequencer, SequencerEvent, Time, Timeline, Track};

// ============================================================================
// Time Struct Tests
// ============================================================================

mod time_tests {
    use super::*;

    #[test]
    fn test_time_from_secs() {
        let t = Time::from_secs(2.5);
        assert_eq!(t.as_secs(), 2.5);
    }

    #[test]
    fn test_time_from_millis() {
        let t = Time::from_millis(1500.0);
        assert_eq!(t.as_secs(), 1.5);
    }

    #[test]
    fn test_time_as_millis() {
        let t = Time::from_secs(2.0);
        assert_eq!(t.as_millis(), 2000.0);
    }

    #[test]
    fn test_time_zero() {
        let t = Time::zero();
        assert!(t.is_zero());
        assert_eq!(t.as_secs(), 0.0);
    }

    #[test]
    fn test_time_is_positive() {
        assert!(Time::from_secs(1.0).is_positive());
        assert!(!Time::from_secs(0.0).is_positive());
        assert!(!Time::from_secs(-1.0).is_positive());
    }

    #[test]
    fn test_time_add_secs() {
        let t = Time::from_secs(1.0);
        let result = t.add_secs(2.5);
        assert_eq!(result.as_secs(), 3.5);
    }

    #[test]
    fn test_time_clamp() {
        let t = Time::from_secs(5.0);
        let clamped = t.clamp(Time::from_secs(2.0), Time::from_secs(4.0));
        assert_eq!(clamped.as_secs(), 4.0);

        let below = Time::from_secs(1.0);
        let clamped_below = below.clamp(Time::from_secs(2.0), Time::from_secs(4.0));
        assert_eq!(clamped_below.as_secs(), 2.0);
    }

    #[test]
    fn test_time_lerp() {
        let a = Time::from_secs(0.0);
        let b = Time::from_secs(10.0);

        assert_eq!(a.lerp(b, 0.0).as_secs(), 0.0);
        assert_eq!(a.lerp(b, 0.5).as_secs(), 5.0);
        assert_eq!(a.lerp(b, 1.0).as_secs(), 10.0);
    }

    #[test]
    fn test_time_add_operator() {
        let a = Time::from_secs(2.0);
        let b = Time::from_secs(3.0);
        assert_eq!((a + b).as_secs(), 5.0);
    }

    #[test]
    fn test_time_sub_operator() {
        let a = Time::from_secs(5.0);
        let b = Time::from_secs(2.0);
        assert_eq!((a - b).as_secs(), 3.0);
    }

    #[test]
    fn test_time_display_seconds() {
        let t = Time::from_secs(2.5);
        let display = format!("{}", t);
        assert!(
            display.contains("2.50s"),
            "Display should show seconds for >=1s: {}",
            display
        );
    }

    #[test]
    fn test_time_display_milliseconds() {
        let t = Time::from_secs(0.5);
        let display = format!("{}", t);
        assert!(
            display.contains("500ms"),
            "Display should show ms for <1s: {}",
            display
        );
    }
}

// ============================================================================
// Track Tests
// ============================================================================

mod track_tests {
    use super::*;

    #[test]
    fn test_track_is_camera() {
        let camera = Track::camera(vec![]);
        assert!(camera.is_camera());
        assert!(!camera.is_animation());
        assert!(!camera.is_audio());
        assert!(!camera.is_fx());
    }

    #[test]
    fn test_track_is_animation() {
        let anim = Track::animation(1, "walk", Time::zero());
        assert!(anim.is_animation());
        assert!(!anim.is_camera());
    }

    #[test]
    fn test_track_is_audio() {
        let audio = Track::audio("music.ogg", Time::zero(), 0.8);
        assert!(audio.is_audio());
        assert!(!audio.is_camera());
    }

    #[test]
    fn test_track_is_fx() {
        let fx = Track::fx("explosion", Time::zero(), serde_json::json!({}));
        assert!(fx.is_fx());
        assert!(!fx.is_camera());
    }

    #[test]
    fn test_track_type_names() {
        assert_eq!(Track::camera(vec![]).type_name(), "Camera");
        assert_eq!(
            Track::animation(1, "clip", Time::zero()).type_name(),
            "Animation"
        );
        assert_eq!(Track::audio("clip", Time::zero(), 1.0).type_name(), "Audio");
        assert_eq!(
            Track::fx("fx", Time::zero(), serde_json::json!({})).type_name(),
            "Fx"
        );
    }

    #[test]
    fn test_track_start_time_camera() {
        let camera = Track::camera(vec![]);
        assert!(
            camera.start_time().is_none(),
            "Camera tracks have no single start time"
        );
    }

    #[test]
    fn test_track_start_time_animation() {
        let anim = Track::animation(1, "walk", Time::from_secs(2.0));
        assert_eq!(anim.start_time().unwrap().as_secs(), 2.0);
    }

    #[test]
    fn test_track_start_time_audio() {
        let audio = Track::audio("music", Time::from_secs(1.5), 1.0);
        assert_eq!(audio.start_time().unwrap().as_secs(), 1.5);
    }

    #[test]
    fn test_track_keyframe_count_camera() {
        let camera = Track::camera(vec![
            CameraKey::at_origin(60.0),
            CameraKey::at_origin(60.0),
            CameraKey::at_origin(60.0),
        ]);
        assert_eq!(camera.keyframe_count(), Some(3));
    }

    #[test]
    fn test_track_keyframe_count_non_camera() {
        let anim = Track::animation(1, "walk", Time::zero());
        assert!(anim.keyframe_count().is_none());
    }
}

// ============================================================================
// CameraKey Tests
// ============================================================================

mod camera_key_tests {
    use super::*;

    #[test]
    fn test_camera_key_creation() {
        let key = CameraKey::new(
            Time::from_secs(1.0),
            (0.0, 5.0, 10.0),
            (0.0, 0.0, 0.0),
            60.0,
        );
        assert_eq!(key.t.as_secs(), 1.0);
        assert_eq!(key.pos, (0.0, 5.0, 10.0));
        assert_eq!(key.look_at, (0.0, 0.0, 0.0));
        assert_eq!(key.fov_deg, 60.0);
    }

    #[test]
    fn test_camera_key_at_origin() {
        let key = CameraKey::at_origin(90.0);
        assert!(key.t.is_zero());
        assert_eq!(key.pos, (0.0, 0.0, 0.0));
        assert_eq!(key.look_at, (0.0, 0.0, -1.0));
        assert_eq!(key.fov_deg, 90.0);
    }

    #[test]
    fn test_camera_key_position() {
        let key = CameraKey::new(Time::zero(), (1.0, 2.0, 3.0), (0.0, 0.0, 0.0), 60.0);
        assert_eq!(key.position(), (1.0, 2.0, 3.0));
    }

    #[test]
    fn test_camera_key_distance_to_target() {
        let key = CameraKey::new(
            Time::zero(),
            (0.0, 0.0, 10.0), // 10 units away on Z
            (0.0, 0.0, 0.0),
            60.0,
        );
        assert!((key.distance_to_target() - 10.0).abs() < 0.001);
    }

    #[test]
    fn test_camera_key_distance_diagonal() {
        let key = CameraKey::new(
            Time::zero(),
            (3.0, 4.0, 0.0), // 3-4-5 triangle
            (0.0, 0.0, 0.0),
            60.0,
        );
        assert!((key.distance_to_target() - 5.0).abs() < 0.001);
    }

    #[test]
    fn test_camera_key_fov_rad() {
        let key = CameraKey::at_origin(90.0);
        assert!((key.fov_rad() - std::f32::consts::FRAC_PI_2).abs() < 0.001);
    }

    // C.7.D: `test_camera_key_is_typical_fov` removed alongside the
    // `is_typical_fov` method (zero production callers; replaced by
    // `sanitize()` at the wider [10°, 170°] canonical range).

    #[test]
    fn test_camera_key_lerp() {
        let a = CameraKey::new(Time::zero(), (0.0, 0.0, 0.0), (0.0, 0.0, -1.0), 60.0);
        let b = CameraKey::new(
            Time::from_secs(2.0),
            (10.0, 10.0, 10.0),
            (10.0, 0.0, -1.0),
            90.0,
        );

        let mid = a.lerp(&b, 0.5);
        assert_eq!(mid.t.as_secs(), 1.0);
        assert_eq!(mid.pos, (5.0, 5.0, 5.0));
        assert_eq!(mid.look_at, (5.0, 0.0, -1.0));
        assert_eq!(mid.fov_deg, 75.0);
    }
}

// ============================================================================
// Timeline Tests
// ============================================================================

mod timeline_tests {
    use super::*;

    #[test]
    fn test_timeline_new() {
        let tl = Timeline::new("intro", 10.0);
        assert_eq!(tl.name, "intro");
        assert_eq!(tl.duration_secs(), 10.0);
        assert!(tl.is_empty());
    }

    #[test]
    fn test_timeline_empty() {
        let tl = Timeline::empty();
        assert!(tl.is_empty());
        assert_eq!(tl.track_count(), 0);
    }

    #[test]
    fn test_timeline_add_track() {
        let mut tl = Timeline::new("test", 5.0);
        tl.add_track(Track::camera(vec![]));
        assert_eq!(tl.track_count(), 1);
        assert!(!tl.is_empty());
    }

    #[test]
    fn test_timeline_camera_track_count() {
        let mut tl = Timeline::new("test", 5.0);
        tl.add_track(Track::camera(vec![]));
        tl.add_track(Track::audio("music", Time::zero(), 1.0));
        tl.add_track(Track::camera(vec![]));

        assert_eq!(tl.camera_track_count(), 2);
        assert_eq!(tl.audio_track_count(), 1);
    }

    #[test]
    fn test_timeline_audio_track_count() {
        let mut tl = Timeline::new("test", 5.0);
        tl.add_track(Track::audio("a", Time::zero(), 1.0));
        tl.add_track(Track::audio("b", Time::zero(), 1.0));

        assert_eq!(tl.audio_track_count(), 2);
    }

    #[test]
    fn test_timeline_animation_track_count() {
        let mut tl = Timeline::new("test", 5.0);
        tl.add_track(Track::animation(1, "walk", Time::zero()));

        assert_eq!(tl.animation_track_count(), 1);
    }

    #[test]
    fn test_timeline_fx_track_count() {
        let mut tl = Timeline::new("test", 5.0);
        tl.add_track(Track::fx("boom", Time::zero(), serde_json::json!({})));

        assert_eq!(tl.fx_track_count(), 1);
    }

    #[test]
    fn test_timeline_total_keyframes() {
        let mut tl = Timeline::new("test", 5.0);
        tl.add_camera_track(vec![CameraKey::at_origin(60.0), CameraKey::at_origin(60.0)]);
        tl.add_camera_track(vec![
            CameraKey::at_origin(60.0),
            CameraKey::at_origin(60.0),
            CameraKey::at_origin(60.0),
        ]);

        assert_eq!(tl.total_keyframes(), 5);
    }

    #[test]
    fn test_timeline_add_audio_track() {
        let mut tl = Timeline::new("test", 5.0);
        tl.add_audio_track("music.ogg", Time::from_secs(1.0), 0.75);

        assert_eq!(tl.audio_track_count(), 1);
    }
}

// ============================================================================
// Sequencer Tests
// ============================================================================

mod sequencer_tests {
    use super::*;

    #[test]
    fn test_sequencer_new() {
        let seq = Sequencer::new();
        assert!(seq.t.is_zero());
    }

    #[test]
    fn test_sequencer_seek() {
        let mut seq = Sequencer::new();
        seq.seek(Time::from_secs(5.0));
        assert_eq!(seq.t.as_secs(), 5.0);
    }

    #[test]
    fn test_sequencer_step_advances_time() {
        let mut seq = Sequencer::new();
        let tl = Timeline::new("test", 10.0);

        seq.step(1.0, &tl).unwrap();
        assert_eq!(seq.t.as_secs(), 1.0);

        seq.step(2.0, &tl).unwrap();
        assert_eq!(seq.t.as_secs(), 3.0);
    }

    #[test]
    fn test_sequencer_step_out_of_range() {
        let mut seq = Sequencer::new();
        let tl = Timeline::new("test", 5.0);

        let result = seq.step(10.0, &tl);
        assert!(result.is_err());
    }

    #[test]
    fn test_sequencer_step_emits_audio_event() {
        let mut seq = Sequencer::new();
        let mut tl = Timeline::new("test", 10.0);
        tl.add_audio_track("music.ogg", Time::from_secs(0.5), 0.8);

        // Step past the audio start
        let events = seq.step(1.0, &tl).unwrap();

        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], SequencerEvent::AudioPlay { .. }));
    }

    #[test]
    fn test_sequencer_step_emits_camera_event() {
        let mut seq = Sequencer::new();
        let mut tl = Timeline::new("test", 10.0);
        tl.add_camera_track(vec![CameraKey::new(
            Time::from_secs(0.5),
            (0.0, 0.0, 0.0),
            (0.0, 0.0, -1.0),
            60.0,
        )]);

        let events = seq.step(1.0, &tl).unwrap();

        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], SequencerEvent::CameraKey(_)));
    }

    #[test]
    fn test_sequencer_step_no_events_before_start() {
        let mut seq = Sequencer::new();
        let mut tl = Timeline::new("test", 10.0);
        tl.add_audio_track("music.ogg", Time::from_secs(5.0), 0.8);

        // Step but not past the audio start
        let events = seq.step(1.0, &tl).unwrap();

        assert!(
            events.is_empty(),
            "No events should be emitted before start time"
        );
    }

    #[test]
    fn test_sequencer_step_animation_event() {
        let mut seq = Sequencer::new();
        let mut tl = Timeline::new("test", 10.0);
        tl.add_track(Track::animation(42, "walk", Time::from_secs(0.5)));

        let events = seq.step(1.0, &tl).unwrap();

        assert_eq!(events.len(), 1);
        match &events[0] {
            SequencerEvent::AnimStart { target, clip } => {
                assert_eq!(*target, 42);
                assert_eq!(clip, "walk");
            }
            _ => panic!("Expected AnimStart event"),
        }
    }

    #[test]
    fn test_sequencer_step_fx_event() {
        let mut seq = Sequencer::new();
        let mut tl = Timeline::new("test", 10.0);
        tl.add_track(Track::fx(
            "explosion",
            Time::from_secs(0.5),
            serde_json::json!({"size": 10}),
        ));

        let events = seq.step(1.0, &tl).unwrap();

        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], SequencerEvent::FxTrigger { .. }));
    }

    #[test]
    fn test_sequencer_multiple_events_same_step() {
        let mut seq = Sequencer::new();
        let mut tl = Timeline::new("test", 10.0);
        tl.add_audio_track("music1.ogg", Time::from_secs(0.3), 1.0);
        tl.add_audio_track("music2.ogg", Time::from_secs(0.6), 1.0);
        tl.add_track(Track::animation(1, "walk", Time::from_secs(0.5)));

        let events = seq.step(1.0, &tl).unwrap();

        assert_eq!(events.len(), 3, "Should emit all events in the step range");
    }
}

// ============================================================================
// Behavioral Correctness Tests
// ============================================================================

mod behavioral_tests {
    use super::*;

    #[test]
    fn test_time_ordering() {
        let a = Time::from_secs(1.0);
        let b = Time::from_secs(2.0);
        assert!(a < b);
        assert!(b > a);
    }

    #[test]
    fn test_time_equality() {
        let a = Time::from_secs(1.5);
        let b = Time::from_secs(1.5);
        assert_eq!(a, b);
    }

    #[test]
    fn test_camera_key_equality() {
        let a = CameraKey::at_origin(60.0);
        let b = CameraKey::at_origin(60.0);
        assert_eq!(a, b);
    }

    #[test]
    fn test_track_equality() {
        let a = Track::audio("music", Time::zero(), 1.0);
        let b = Track::audio("music", Time::zero(), 1.0);
        assert_eq!(a, b);
    }

    #[test]
    fn test_sequencer_events_ordered_by_occurrence() {
        // Events should be emitted in track order, not time order
        let mut seq = Sequencer::new();
        let mut tl = Timeline::new("test", 10.0);
        tl.add_audio_track("first", Time::from_secs(0.8), 1.0);
        tl.add_audio_track("second", Time::from_secs(0.2), 1.0);

        let events = seq.step(1.0, &tl).unwrap();

        // Both events are in range and emitted
        assert_eq!(events.len(), 2);
    }

    #[test]
    fn test_timeline_duration_preserved() {
        let tl = Timeline::new("test", 30.0);
        assert_eq!(tl.duration.as_secs(), 30.0);
        assert_eq!(tl.duration_secs(), 30.0);
    }
}
