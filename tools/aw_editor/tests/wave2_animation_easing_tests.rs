//! Wave 2 Mutation Remediation — Animation Panel: Easing, Tracks, Clips, Panel State
//!
//! Targets: animation.rs (1,576 lines)
//! - PlaybackState (3 variants × name/icon/is_active/Display)
//! - AnimatedProperty (5 variants × name/icon/is_position/is_rotation/Display)
//! - AnimationTrack: evaluate() with all 12 easing functions
//! - AnimationClip: sample_bounce/spin/pulse factories
//! - AnimationPanel: update state machine, output accumulation

use astract::animation::EasingFunction;
use aw_editor_lib::panels::animation::{
    AnimatedProperty, AnimationClip, AnimationOutput, AnimationPanel, AnimationTrack, Keyframe,
    PlaybackState,
};

// ============================================================================
// PLAYBACK STATE — ENUM VARIANTS
// ============================================================================

#[test]
fn playback_state_stopped_name() {
    assert_eq!(PlaybackState::Stopped.name(), "Stopped");
}

#[test]
fn playback_state_playing_name() {
    assert_eq!(PlaybackState::Playing.name(), "Playing");
}

#[test]
fn playback_state_paused_name() {
    assert_eq!(PlaybackState::Paused.name(), "Paused");
}

#[test]
fn playback_state_stopped_icon() {
    assert_eq!(PlaybackState::Stopped.icon(), "[]");
}

#[test]
fn playback_state_playing_icon() {
    assert_eq!(PlaybackState::Playing.icon(), ">");
}

#[test]
fn playback_state_paused_icon() {
    assert_eq!(PlaybackState::Paused.icon(), "||");
}

#[test]
fn playback_state_all_count() {
    assert_eq!(PlaybackState::all().len(), 3);
}

#[test]
fn playback_state_all_contains_all_variants() {
    let all = PlaybackState::all();
    assert!(all.contains(&PlaybackState::Stopped));
    assert!(all.contains(&PlaybackState::Playing));
    assert!(all.contains(&PlaybackState::Paused));
}

#[test]
fn playback_state_is_active_stopped() {
    assert!(!PlaybackState::Stopped.is_active());
}

#[test]
fn playback_state_is_active_playing() {
    assert!(PlaybackState::Playing.is_active());
}

#[test]
fn playback_state_is_active_paused() {
    assert!(!PlaybackState::Paused.is_active());
}

#[test]
fn playback_state_default_is_stopped() {
    assert_eq!(PlaybackState::default(), PlaybackState::Stopped);
}

#[test]
fn playback_state_display_stopped() {
    let s = format!("{}", PlaybackState::Stopped);
    assert!(s.contains("[]"));
    assert!(s.contains("Stopped"));
}

#[test]
fn playback_state_display_playing() {
    let s = format!("{}", PlaybackState::Playing);
    assert!(s.contains(">"));
    assert!(s.contains("Playing"));
}

#[test]
fn playback_state_display_paused() {
    let s = format!("{}", PlaybackState::Paused);
    assert!(s.contains("||"));
    assert!(s.contains("Paused"));
}

#[test]
fn playback_state_eq() {
    assert_eq!(PlaybackState::Playing, PlaybackState::Playing);
    assert_ne!(PlaybackState::Playing, PlaybackState::Stopped);
}

#[test]
fn playback_state_hash() {
    use std::collections::HashSet;
    let mut set = HashSet::new();
    set.insert(PlaybackState::Playing);
    set.insert(PlaybackState::Stopped);
    set.insert(PlaybackState::Playing); // duplicate
    assert_eq!(set.len(), 2);
}

// ============================================================================
// ANIMATED PROPERTY — ENUM VARIANTS
// ============================================================================

#[test]
fn animated_property_position_x_name() {
    assert_eq!(AnimatedProperty::PositionX.name(), "Position X");
}

#[test]
fn animated_property_position_y_name() {
    assert_eq!(AnimatedProperty::PositionY.name(), "Position Y");
}

#[test]
fn animated_property_position_z_name() {
    assert_eq!(AnimatedProperty::PositionZ.name(), "Position Z");
}

#[test]
fn animated_property_rotation_y_name() {
    assert_eq!(AnimatedProperty::RotationY.name(), "Rotation Y");
}

#[test]
fn animated_property_scale_name() {
    assert_eq!(AnimatedProperty::Scale.name(), "Scale");
}

#[test]
fn animated_property_position_x_icon() {
    assert_eq!(AnimatedProperty::PositionX.icon(), "↔");
}

#[test]
fn animated_property_position_y_icon() {
    assert_eq!(AnimatedProperty::PositionY.icon(), "↕");
}

#[test]
fn animated_property_position_z_icon() {
    assert_eq!(AnimatedProperty::PositionZ.icon(), "↗");
}

#[test]
fn animated_property_rotation_y_icon() {
    assert_eq!(AnimatedProperty::RotationY.icon(), "[Sync]");
}

#[test]
fn animated_property_scale_icon() {
    assert_eq!(AnimatedProperty::Scale.icon(), "[Sq]");
}

#[test]
fn animated_property_all_count() {
    assert_eq!(AnimatedProperty::all().len(), 5);
}

#[test]
fn animated_property_all_contains_all_variants() {
    let all = AnimatedProperty::all();
    assert!(all.contains(&AnimatedProperty::PositionX));
    assert!(all.contains(&AnimatedProperty::PositionY));
    assert!(all.contains(&AnimatedProperty::PositionZ));
    assert!(all.contains(&AnimatedProperty::RotationY));
    assert!(all.contains(&AnimatedProperty::Scale));
}

#[test]
fn animated_property_is_position_x() {
    assert!(AnimatedProperty::PositionX.is_position());
}

#[test]
fn animated_property_is_position_y() {
    assert!(AnimatedProperty::PositionY.is_position());
}

#[test]
fn animated_property_is_position_z() {
    assert!(AnimatedProperty::PositionZ.is_position());
}

#[test]
fn animated_property_rotation_not_position() {
    assert!(!AnimatedProperty::RotationY.is_position());
}

#[test]
fn animated_property_scale_not_position() {
    assert!(!AnimatedProperty::Scale.is_position());
}

#[test]
fn animated_property_is_rotation_y() {
    assert!(AnimatedProperty::RotationY.is_rotation());
}

#[test]
fn animated_property_position_x_not_rotation() {
    assert!(!AnimatedProperty::PositionX.is_rotation());
}

#[test]
fn animated_property_scale_not_rotation() {
    assert!(!AnimatedProperty::Scale.is_rotation());
}

#[test]
fn animated_property_default_is_position_y() {
    assert_eq!(AnimatedProperty::default(), AnimatedProperty::PositionY);
}

#[test]
fn animated_property_display_position_x() {
    let s = format!("{}", AnimatedProperty::PositionX);
    assert!(s.contains("↔"));
    assert!(s.contains("Position X"));
}

#[test]
fn animated_property_display_scale() {
    let s = format!("{}", AnimatedProperty::Scale);
    assert!(s.contains("[Sq]"));
    assert!(s.contains("Scale"));
}

// ============================================================================
// ANIMATION TRACK — EVALUATE EMPTY / SINGLE / INTERPOLATION
// ============================================================================

#[test]
fn track_new_has_correct_property() {
    let track = AnimationTrack::new(AnimatedProperty::PositionX);
    assert_eq!(track.property, AnimatedProperty::PositionX);
    assert!(track.keyframes.is_empty());
}

#[test]
fn track_evaluate_empty_position_returns_zero() {
    let track = AnimationTrack::new(AnimatedProperty::PositionX);
    assert_eq!(track.evaluate(0.0), 0.0);
    assert_eq!(track.evaluate(5.0), 0.0);
}

#[test]
fn track_evaluate_empty_scale_returns_one() {
    let track = AnimationTrack::new(AnimatedProperty::Scale);
    assert_eq!(track.evaluate(0.0), 1.0);
    assert_eq!(track.evaluate(100.0), 1.0);
}

#[test]
fn track_evaluate_empty_position_y_returns_zero() {
    let track = AnimationTrack::new(AnimatedProperty::PositionY);
    assert_eq!(track.evaluate(0.5), 0.0);
}

#[test]
fn track_evaluate_empty_rotation_returns_zero() {
    let track = AnimationTrack::new(AnimatedProperty::RotationY);
    assert_eq!(track.evaluate(1.0), 0.0);
}

#[test]
fn track_evaluate_single_keyframe_returns_value() {
    let mut track = AnimationTrack::new(AnimatedProperty::PositionY);
    track.keyframes.push(Keyframe {
        time: 0.0,
        value: 42.0,
        easing: EasingFunction::Linear,
    });
    assert_eq!(track.evaluate(0.0), 42.0);
    assert_eq!(track.evaluate(1.0), 42.0);
    assert_eq!(track.evaluate(-1.0), 42.0);
}

#[test]
fn track_evaluate_linear_interpolation_midpoint() {
    let mut track = AnimationTrack::new(AnimatedProperty::PositionY);
    track.keyframes.push(Keyframe {
        time: 0.0,
        value: 0.0,
        easing: EasingFunction::Linear,
    });
    track.keyframes.push(Keyframe {
        time: 1.0,
        value: 10.0,
        easing: EasingFunction::Linear,
    });
    let v = track.evaluate(0.5);
    assert!((v - 5.0).abs() < 0.001, "Expected ~5.0, got {}", v);
}

#[test]
fn track_evaluate_linear_at_start() {
    let mut track = AnimationTrack::new(AnimatedProperty::PositionY);
    track.keyframes.push(Keyframe {
        time: 0.0,
        value: 0.0,
        easing: EasingFunction::Linear,
    });
    track.keyframes.push(Keyframe {
        time: 1.0,
        value: 10.0,
        easing: EasingFunction::Linear,
    });
    let v = track.evaluate(0.0);
    assert!((v - 0.0).abs() < 0.001);
}

#[test]
fn track_evaluate_linear_at_end() {
    let mut track = AnimationTrack::new(AnimatedProperty::PositionY);
    track.keyframes.push(Keyframe {
        time: 0.0,
        value: 0.0,
        easing: EasingFunction::Linear,
    });
    track.keyframes.push(Keyframe {
        time: 1.0,
        value: 10.0,
        easing: EasingFunction::Linear,
    });
    let v = track.evaluate(1.0);
    assert!((v - 10.0).abs() < 0.001);
}

#[test]
fn track_evaluate_before_first_keyframe() {
    let mut track = AnimationTrack::new(AnimatedProperty::PositionY);
    track.keyframes.push(Keyframe {
        time: 1.0,
        value: 5.0,
        easing: EasingFunction::Linear,
    });
    track.keyframes.push(Keyframe {
        time: 2.0,
        value: 10.0,
        easing: EasingFunction::Linear,
    });
    // Before first keyframe, should clamp to first value
    let v = track.evaluate(0.0);
    assert!((v - 5.0).abs() < 0.1, "Expected ~5.0, got {}", v);
}

#[test]
fn track_evaluate_after_last_keyframe() {
    let mut track = AnimationTrack::new(AnimatedProperty::PositionY);
    track.keyframes.push(Keyframe {
        time: 0.0,
        value: 0.0,
        easing: EasingFunction::Linear,
    });
    track.keyframes.push(Keyframe {
        time: 1.0,
        value: 10.0,
        easing: EasingFunction::Linear,
    });
    // After last keyframe, should return last value
    let v = track.evaluate(5.0);
    assert!((v - 10.0).abs() < 0.001);
}

// ============================================================================
// EASING FUNCTION TESTS — each function at t=0.0, t=0.5, t=1.0
// ============================================================================

fn make_track_with_easing(easing: EasingFunction) -> AnimationTrack {
    let mut track = AnimationTrack::new(AnimatedProperty::PositionY);
    track.keyframes.push(Keyframe {
        time: 0.0,
        value: 0.0,
        easing,
    });
    track.keyframes.push(Keyframe {
        time: 1.0,
        value: 1.0,
        easing: EasingFunction::Linear,
    });
    track
}

#[test]
fn easing_linear_at_zero() {
    let track = make_track_with_easing(EasingFunction::Linear);
    assert!((track.evaluate(0.0) - 0.0).abs() < 0.001);
}

#[test]
fn easing_linear_at_half() {
    let track = make_track_with_easing(EasingFunction::Linear);
    assert!((track.evaluate(0.5) - 0.5).abs() < 0.001);
}

#[test]
fn easing_linear_at_one() {
    let track = make_track_with_easing(EasingFunction::Linear);
    assert!((track.evaluate(1.0) - 1.0).abs() < 0.001);
}

#[test]
fn easing_quad_in_at_half() {
    let track = make_track_with_easing(EasingFunction::QuadIn);
    let v = track.evaluate(0.5);
    // QuadIn: t*t => 0.25
    assert!(
        (v - 0.25).abs() < 0.001,
        "QuadIn(0.5) expected 0.25, got {}",
        v
    );
}

#[test]
fn easing_quad_in_at_zero() {
    let track = make_track_with_easing(EasingFunction::QuadIn);
    assert!((track.evaluate(0.0) - 0.0).abs() < 0.001);
}

#[test]
fn easing_quad_in_at_one() {
    let track = make_track_with_easing(EasingFunction::QuadIn);
    assert!((track.evaluate(1.0) - 1.0).abs() < 0.001);
}

#[test]
fn easing_quad_out_at_half() {
    let track = make_track_with_easing(EasingFunction::QuadOut);
    let v = track.evaluate(0.5);
    // QuadOut: 1 - (1-t)^2 = 1 - 0.25 = 0.75
    assert!(
        (v - 0.75).abs() < 0.001,
        "QuadOut(0.5) expected 0.75, got {}",
        v
    );
}

#[test]
fn easing_quad_out_at_zero() {
    let track = make_track_with_easing(EasingFunction::QuadOut);
    assert!((track.evaluate(0.0) - 0.0).abs() < 0.001);
}

#[test]
fn easing_quad_out_at_one() {
    let track = make_track_with_easing(EasingFunction::QuadOut);
    assert!((track.evaluate(1.0) - 1.0).abs() < 0.001);
}

#[test]
fn easing_quad_in_out_first_half() {
    let track = make_track_with_easing(EasingFunction::QuadInOut);
    let v = track.evaluate(0.25);
    // t < 0.5 => 2*t*t = 2*0.0625 = 0.125
    assert!(
        (v - 0.125).abs() < 0.01,
        "QuadInOut(0.25) expected 0.125, got {}",
        v
    );
}

#[test]
fn easing_quad_in_out_at_half() {
    let track = make_track_with_easing(EasingFunction::QuadInOut);
    let v = track.evaluate(0.5);
    // boundary: 2*0.5*0.5 = 0.5
    assert!(
        (v - 0.5).abs() < 0.01,
        "QuadInOut(0.5) expected 0.5, got {}",
        v
    );
}

#[test]
fn easing_quad_in_out_second_half() {
    let track = make_track_with_easing(EasingFunction::QuadInOut);
    let v = track.evaluate(0.75);
    // t >= 0.5 => 1 - (-2*0.75+2)^2/2 = 1 - (0.5)^2/2 = 1 - 0.125 = 0.875
    assert!(
        (v - 0.875).abs() < 0.01,
        "QuadInOut(0.75) expected 0.875, got {}",
        v
    );
}

#[test]
fn easing_cubic_in_at_half() {
    let track = make_track_with_easing(EasingFunction::CubicIn);
    let v = track.evaluate(0.5);
    // CubicIn: t^3 = 0.125
    assert!(
        (v - 0.125).abs() < 0.001,
        "CubicIn(0.5) expected 0.125, got {}",
        v
    );
}

#[test]
fn easing_cubic_in_at_one() {
    let track = make_track_with_easing(EasingFunction::CubicIn);
    assert!((track.evaluate(1.0) - 1.0).abs() < 0.001);
}

#[test]
fn easing_cubic_out_at_half() {
    let track = make_track_with_easing(EasingFunction::CubicOut);
    let v = track.evaluate(0.5);
    // CubicOut: 1 - (1-0.5)^3 = 1 - 0.125 = 0.875
    assert!(
        (v - 0.875).abs() < 0.001,
        "CubicOut(0.5) expected 0.875, got {}",
        v
    );
}

#[test]
fn easing_cubic_out_at_zero() {
    let track = make_track_with_easing(EasingFunction::CubicOut);
    assert!((track.evaluate(0.0) - 0.0).abs() < 0.001);
}

#[test]
fn easing_sine_in_at_half() {
    let track = make_track_with_easing(EasingFunction::SineIn);
    let v = track.evaluate(0.5);
    // SineIn: 1 - cos(t * PI/2) = 1 - cos(PI/4) ≈ 1 - 0.7071 = 0.2929
    assert!(
        (v - 0.2929).abs() < 0.01,
        "SineIn(0.5) expected ~0.29, got {}",
        v
    );
}

#[test]
fn easing_sine_in_at_one() {
    let track = make_track_with_easing(EasingFunction::SineIn);
    assert!((track.evaluate(1.0) - 1.0).abs() < 0.001);
}

#[test]
fn easing_sine_out_at_half() {
    let track = make_track_with_easing(EasingFunction::SineOut);
    let v = track.evaluate(0.5);
    // SineOut: sin(t * PI/2) = sin(PI/4) ≈ 0.7071
    assert!(
        (v - 0.7071).abs() < 0.01,
        "SineOut(0.5) expected ~0.71, got {}",
        v
    );
}

#[test]
fn easing_sine_out_at_zero() {
    let track = make_track_with_easing(EasingFunction::SineOut);
    assert!((track.evaluate(0.0) - 0.0).abs() < 0.001);
}

#[test]
fn easing_sine_in_out_at_half() {
    let track = make_track_with_easing(EasingFunction::SineInOut);
    let v = track.evaluate(0.5);
    // SineInOut: -(cos(PI*t)-1)/2 = -(cos(PI/2)-1)/2 = -(0-1)/2 = 0.5
    assert!(
        (v - 0.5).abs() < 0.01,
        "SineInOut(0.5) expected 0.5, got {}",
        v
    );
}

#[test]
fn easing_sine_in_out_at_zero() {
    let track = make_track_with_easing(EasingFunction::SineInOut);
    assert!((track.evaluate(0.0) - 0.0).abs() < 0.001);
}

#[test]
fn easing_sine_in_out_at_one() {
    let track = make_track_with_easing(EasingFunction::SineInOut);
    assert!((track.evaluate(1.0) - 1.0).abs() < 0.001);
}

#[test]
fn easing_elastic_out_at_zero() {
    let track = make_track_with_easing(EasingFunction::ElasticOut);
    assert!((track.evaluate(0.0) - 0.0).abs() < 0.001);
}

#[test]
fn easing_elastic_out_at_one() {
    let track = make_track_with_easing(EasingFunction::ElasticOut);
    assert!((track.evaluate(1.0) - 1.0).abs() < 0.001);
}

#[test]
fn easing_elastic_out_at_half() {
    let track = make_track_with_easing(EasingFunction::ElasticOut);
    let v = track.evaluate(0.5);
    // ElasticOut overshoots: 2^(-10*0.5) * sin((5*10-0.75)*2PI/3) + 1
    // ≈ 2^(-5) * sin(...) + 1 ≈ 0.03125 * sin(~) + 1
    // Should be > 0.9 and roughly close to 1.0
    assert!(v > 0.8, "ElasticOut(0.5) expected >0.8, got {}", v);
}

#[test]
fn easing_bounce_out_at_zero() {
    let track = make_track_with_easing(EasingFunction::BounceOut);
    assert!((track.evaluate(0.0) - 0.0).abs() < 0.001);
}

#[test]
fn easing_bounce_out_at_one() {
    let track = make_track_with_easing(EasingFunction::BounceOut);
    assert!((track.evaluate(1.0) - 1.0).abs() < 0.001);
}

#[test]
fn easing_bounce_out_first_region() {
    // t < 1/2.75 ≈ 0.3636 → n1*t*t
    let track = make_track_with_easing(EasingFunction::BounceOut);
    let v = track.evaluate(0.2);
    // 7.5625 * 0.04 = 0.3025
    assert!(
        (v - 0.3025).abs() < 0.05,
        "BounceOut(0.2) expected ~0.30, got {}",
        v
    );
}

#[test]
fn easing_bounce_out_second_region() {
    // t ∈ [1/2.75, 2/2.75) → [0.3636, 0.7273)
    let track = make_track_with_easing(EasingFunction::BounceOut);
    let v = track.evaluate(0.5);
    // In second bounce region
    assert!(
        v > 0.5 && v < 1.0,
        "BounceOut(0.5) expected (0.5,1.0), got {}",
        v
    );
}

#[test]
fn easing_bounce_out_third_region() {
    // t ∈ [2/2.75, 2.5/2.75) → [0.7273, 0.9091)
    let track = make_track_with_easing(EasingFunction::BounceOut);
    let v = track.evaluate(0.8);
    assert!(v > 0.8, "BounceOut(0.8) expected >0.8, got {}", v);
}

#[test]
fn easing_bounce_out_fourth_region() {
    // t >= 2.5/2.75 ≈ 0.909
    let track = make_track_with_easing(EasingFunction::BounceOut);
    let v = track.evaluate(0.95);
    assert!(v > 0.95, "BounceOut(0.95) expected >0.95, got {}", v);
}

// ============================================================================
// MULTI-KEYFRAME INTERPOLATION
// ============================================================================

#[test]
fn track_three_keyframes_first_segment() {
    let mut track = AnimationTrack::new(AnimatedProperty::PositionY);
    track.keyframes = vec![
        Keyframe {
            time: 0.0,
            value: 0.0,
            easing: EasingFunction::Linear,
        },
        Keyframe {
            time: 1.0,
            value: 10.0,
            easing: EasingFunction::Linear,
        },
        Keyframe {
            time: 2.0,
            value: 5.0,
            easing: EasingFunction::Linear,
        },
    ];
    let v = track.evaluate(0.5);
    assert!((v - 5.0).abs() < 0.01);
}

#[test]
fn track_three_keyframes_second_segment() {
    let mut track = AnimationTrack::new(AnimatedProperty::PositionY);
    track.keyframes = vec![
        Keyframe {
            time: 0.0,
            value: 0.0,
            easing: EasingFunction::Linear,
        },
        Keyframe {
            time: 1.0,
            value: 10.0,
            easing: EasingFunction::Linear,
        },
        Keyframe {
            time: 2.0,
            value: 5.0,
            easing: EasingFunction::Linear,
        },
    ];
    let v = track.evaluate(1.5);
    assert!((v - 7.5).abs() < 0.01, "Expected ~7.5, got {}", v);
}

#[test]
fn track_negative_values() {
    let mut track = AnimationTrack::new(AnimatedProperty::PositionY);
    track.keyframes = vec![
        Keyframe {
            time: 0.0,
            value: -10.0,
            easing: EasingFunction::Linear,
        },
        Keyframe {
            time: 1.0,
            value: 10.0,
            easing: EasingFunction::Linear,
        },
    ];
    let v = track.evaluate(0.5);
    assert!((v - 0.0).abs() < 0.01);
}

// ============================================================================
// ANIMATION CLIP — FACTORIES
// ============================================================================

#[test]
fn clip_default_name() {
    let clip = AnimationClip::default();
    assert_eq!(clip.name, "New Animation");
}

#[test]
fn clip_default_duration() {
    let clip = AnimationClip::default();
    assert_eq!(clip.duration, 2.0);
}

#[test]
fn clip_default_not_looping() {
    let clip = AnimationClip::default();
    assert!(!clip.looping);
}

#[test]
fn clip_default_no_tracks() {
    let clip = AnimationClip::default();
    assert!(clip.tracks.is_empty());
}

#[test]
fn clip_sample_bounce_name() {
    let clip = AnimationClip::sample_bounce();
    assert_eq!(clip.name, "Bounce");
}

#[test]
fn clip_sample_bounce_duration() {
    let clip = AnimationClip::sample_bounce();
    assert_eq!(clip.duration, 1.0);
}

#[test]
fn clip_sample_bounce_looping() {
    let clip = AnimationClip::sample_bounce();
    assert!(clip.looping);
}

#[test]
fn clip_sample_bounce_has_one_track() {
    let clip = AnimationClip::sample_bounce();
    assert_eq!(clip.tracks.len(), 1);
}

#[test]
fn clip_sample_bounce_track_property() {
    let clip = AnimationClip::sample_bounce();
    assert_eq!(clip.tracks[0].property, AnimatedProperty::PositionY);
}

#[test]
fn clip_sample_bounce_has_three_keyframes() {
    let clip = AnimationClip::sample_bounce();
    assert_eq!(clip.tracks[0].keyframes.len(), 3);
}

#[test]
fn clip_sample_bounce_keyframe_values() {
    let clip = AnimationClip::sample_bounce();
    let kfs = &clip.tracks[0].keyframes;
    assert_eq!(kfs[0].value, 0.0);
    assert_eq!(kfs[1].value, 2.0);
    assert_eq!(kfs[2].value, 0.0);
}

#[test]
fn clip_sample_spin_name() {
    let clip = AnimationClip::sample_spin();
    assert_eq!(clip.name, "Spin");
}

#[test]
fn clip_sample_spin_duration() {
    let clip = AnimationClip::sample_spin();
    assert_eq!(clip.duration, 2.0);
}

#[test]
fn clip_sample_spin_looping() {
    let clip = AnimationClip::sample_spin();
    assert!(clip.looping);
}

#[test]
fn clip_sample_spin_track_property() {
    let clip = AnimationClip::sample_spin();
    assert_eq!(clip.tracks[0].property, AnimatedProperty::RotationY);
}

#[test]
fn clip_sample_spin_rotation_range() {
    let clip = AnimationClip::sample_spin();
    let kfs = &clip.tracks[0].keyframes;
    assert_eq!(kfs[0].value, 0.0);
    assert!((kfs[1].value - std::f32::consts::TAU).abs() < 0.001);
}

#[test]
fn clip_sample_pulse_name() {
    let clip = AnimationClip::sample_pulse();
    assert_eq!(clip.name, "Pulse");
}

#[test]
fn clip_sample_pulse_duration() {
    let clip = AnimationClip::sample_pulse();
    assert_eq!(clip.duration, 1.0);
}

#[test]
fn clip_sample_pulse_looping() {
    let clip = AnimationClip::sample_pulse();
    assert!(clip.looping);
}

#[test]
fn clip_sample_pulse_track_property() {
    let clip = AnimationClip::sample_pulse();
    assert_eq!(clip.tracks[0].property, AnimatedProperty::Scale);
}

#[test]
fn clip_sample_pulse_keyframe_values() {
    let clip = AnimationClip::sample_pulse();
    let kfs = &clip.tracks[0].keyframes;
    assert_eq!(kfs[0].value, 1.0);
    assert_eq!(kfs[1].value, 1.3);
    assert_eq!(kfs[2].value, 1.0);
}

#[test]
fn clip_sample_pulse_keyframe_easing() {
    let clip = AnimationClip::sample_pulse();
    for kf in &clip.tracks[0].keyframes {
        assert_eq!(kf.easing, EasingFunction::SineInOut);
    }
}

// ============================================================================
// ANIMATION OUTPUT
// ============================================================================

#[test]
fn animation_output_default() {
    let out = AnimationOutput::default();
    assert_eq!(out.rotation_y, 0.0);
    assert_eq!(out.scale_multiplier, 0.0); // f32 default is 0.0
}

// ============================================================================
// ANIMATION PANEL — STATE
// ============================================================================

#[test]
fn panel_default_is_stopped() {
    let panel = AnimationPanel::default();
    assert_eq!(panel.playback_state, PlaybackState::Stopped);
}

#[test]
fn panel_default_time_zero() {
    let panel = AnimationPanel::default();
    assert_eq!(panel.current_time, 0.0);
}

#[test]
fn panel_default_speed_one() {
    let panel = AnimationPanel::default();
    assert_eq!(panel.playback_speed, 1.0);
}

#[test]
fn panel_default_no_selected_entity() {
    let panel = AnimationPanel::default();
    assert!(panel.selected_entity.is_none());
}

#[test]
fn panel_default_has_sample_clips() {
    let panel = AnimationPanel::default();
    assert_eq!(panel.clips.len(), 3);
    assert_eq!(panel.clips[0].name, "Bounce");
    assert_eq!(panel.clips[1].name, "Spin");
    assert_eq!(panel.clips[2].name, "Pulse");
}

#[test]
fn panel_default_selected_clip_idx() {
    let panel = AnimationPanel::default();
    assert_eq!(panel.selected_clip_idx, Some(0));
}

#[test]
fn panel_default_show_editor() {
    let panel = AnimationPanel::default();
    assert!(panel.show_editor);
}

#[test]
fn panel_is_playing_when_stopped() {
    let panel = AnimationPanel::default();
    assert!(!panel.is_playing());
}

#[test]
fn panel_is_playing_when_playing() {
    let mut panel = AnimationPanel::default();
    panel.playback_state = PlaybackState::Playing;
    assert!(panel.is_playing());
}

#[test]
fn panel_set_selected_entity() {
    let mut panel = AnimationPanel::default();
    panel.set_selected_entity(Some(42));
    assert_eq!(panel.selected_entity, Some(42));
}

#[test]
fn panel_set_selected_entity_none() {
    let mut panel = AnimationPanel::default();
    panel.set_selected_entity(Some(1));
    panel.set_selected_entity(None);
    assert!(panel.selected_entity.is_none());
}

#[test]
fn panel_get_output_default() {
    let panel = AnimationPanel::default();
    let out = panel.get_output();
    assert_eq!(out.scale_multiplier, 1.0);
}

#[test]
fn panel_update_while_stopped_returns_none() {
    let mut panel = AnimationPanel::default();
    assert!(panel.update(0.016).is_none());
}

#[test]
fn panel_update_while_paused_returns_none() {
    let mut panel = AnimationPanel::default();
    panel.playback_state = PlaybackState::Paused;
    assert!(panel.update(0.016).is_none());
}

#[test]
fn panel_update_while_playing_returns_some() {
    let mut panel = AnimationPanel::default();
    panel.playback_state = PlaybackState::Playing;
    let result = panel.update(0.016);
    assert!(result.is_some());
}

#[test]
fn panel_update_advances_time() {
    let mut panel = AnimationPanel::default();
    panel.playback_state = PlaybackState::Playing;
    panel.current_time = 0.0;
    panel.update(0.1);
    assert!(panel.current_time > 0.0);
}

#[test]
fn panel_update_looping_wraps_time() {
    let mut panel = AnimationPanel::default();
    panel.playback_state = PlaybackState::Playing;
    // Bounce clip is selected (idx 0), duration 1.0, looping=true
    panel.current_time = 0.9;
    panel.update(0.2); // 0.9 + 0.2 = 1.1 > 1.0 → wraps
    assert!(
        panel.current_time < 1.0,
        "Should wrap, got {}",
        panel.current_time
    );
    assert!(panel.current_time >= 0.0);
}

#[test]
fn panel_update_non_looping_stops() {
    let mut panel = AnimationPanel::default();
    panel.playback_state = PlaybackState::Playing;
    // Replace clip 0 with non-looping version
    panel.clips[0].looping = false;
    panel.current_time = 0.95;
    panel.update(0.1); // 0.95 + 0.1 = 1.05 > 1.0 → clamp + stop
    assert_eq!(panel.current_time, panel.clips[0].duration);
    assert_eq!(panel.playback_state, PlaybackState::Stopped);
}

#[test]
fn panel_update_no_clip_selected_returns_none() {
    let mut panel = AnimationPanel::default();
    panel.playback_state = PlaybackState::Playing;
    panel.selected_clip_idx = None;
    assert!(panel.update(0.016).is_none());
}

#[test]
fn panel_output_scale_from_pulse() {
    let mut panel = AnimationPanel::default();
    panel.playback_state = PlaybackState::Playing;
    panel.selected_clip_idx = Some(2); // Pulse clip (Scale track)
    panel.current_time = 0.0;
    let out = panel.update(0.01);
    assert!(out.is_some());
    let out = out.unwrap();
    // Scale should be close to 1.0 at the start
    assert!((out.scale_multiplier - 1.0).abs() < 0.1);
}

#[test]
fn panel_output_rotation_from_spin() {
    let mut panel = AnimationPanel::default();
    panel.playback_state = PlaybackState::Playing;
    panel.selected_clip_idx = Some(1); // Spin clip (RotationY track)
    panel.current_time = 0.0;
    let out = panel.update(1.0); // mid‐way through 2s clip
    assert!(out.is_some());
    let out = out.unwrap();
    // Rotation should be approximately TAU/2 at halfway
    assert!(
        out.rotation_y > 0.0,
        "Rotation should be positive, got {}",
        out.rotation_y
    );
}

#[test]
fn panel_playback_speed_scales_time() {
    let mut panel = AnimationPanel::default();
    panel.playback_state = PlaybackState::Playing;
    panel.playback_speed = 2.0;
    panel.current_time = 0.0;
    panel.update(0.1); // should advance by 0.2
    assert!((panel.current_time - 0.2).abs() < 0.01);
}
