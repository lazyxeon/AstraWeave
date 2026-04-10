//! Mutation-Resistant Tests for Cinematics Panel APIs
//!
//! This module provides comprehensive mutation-resistant tests for:
//! - TrackType (6 variants: Camera, Animation, Audio, Fx, Dialogue, Event)
//! - CameraInterpolation (5 variants with is_smooth() method)
//! - PlaybackState (4 variants with is_running() method)
//! - PlaybackSpeed (5 variants with multiplier() method)
//! - CinematicsTab (6 variants)
//!
//! Test patterns:
//! 1. Boolean return path tests - verify true AND false paths
//! 2. Match arm coverage - test every enum variant
//! 3. Boundary conditions - test edge cases
//! 4. Comparison operator tests - verify comparisons work correctly

use aw_editor_lib::panels::cinematics_panel::PlaybackState;
use aw_editor_lib::panels::{CameraInterpolation, CinematicsTab, PlaybackSpeed, TrackType};

// ============================================================================
// TrackType Tests - 6 variants
// ============================================================================

mod track_type_tests {
    use super::*;

    // Name tests - verify each variant returns its expected name
    #[test]
    fn track_type_camera_name() {
        assert_eq!(TrackType::Camera.name(), "Camera");
    }

    #[test]
    fn track_type_animation_name() {
        assert_eq!(TrackType::Animation.name(), "Animation");
    }

    #[test]
    fn track_type_audio_name() {
        assert_eq!(TrackType::Audio.name(), "Audio");
    }

    #[test]
    fn track_type_fx_name() {
        assert_eq!(TrackType::Fx.name(), "VFX");
    }

    #[test]
    fn track_type_dialogue_name() {
        assert_eq!(TrackType::Dialogue.name(), "Dialogue");
    }

    #[test]
    fn track_type_event_name() {
        assert_eq!(TrackType::Event.name(), "Event");
    }

    // Icon tests - verify each variant returns correct icon
    #[test]
    fn track_type_camera_icon() {
        assert_eq!(TrackType::Camera.icon(), "📷");
    }

    #[test]
    fn track_type_animation_icon() {
        assert_eq!(TrackType::Animation.icon(), "[Anim]");
    }

    #[test]
    fn track_type_audio_icon() {
        assert_eq!(TrackType::Audio.icon(), "[Snd]");
    }

    #[test]
    fn track_type_fx_icon() {
        assert_eq!(TrackType::Fx.icon(), "[Fx]");
    }

    #[test]
    fn track_type_dialogue_icon() {
        assert_eq!(TrackType::Dialogue.icon(), "[Chat]");
    }

    #[test]
    fn track_type_event_icon() {
        assert_eq!(TrackType::Event.icon(), "[Zap]");
    }

    // Display tests - verify Display trait
    #[test]
    fn track_type_camera_display() {
        assert_eq!(TrackType::Camera.to_string(), "📷 Camera");
    }

    #[test]
    fn track_type_animation_display() {
        assert_eq!(TrackType::Animation.to_string(), "[Anim] Animation");
    }

    #[test]
    fn track_type_audio_display() {
        assert_eq!(TrackType::Audio.to_string(), "[Snd] Audio");
    }

    #[test]
    fn track_type_fx_display() {
        assert_eq!(TrackType::Fx.to_string(), "[Fx] VFX");
    }

    #[test]
    fn track_type_dialogue_display() {
        assert_eq!(TrackType::Dialogue.to_string(), "[Chat] Dialogue");
    }

    #[test]
    fn track_type_event_display() {
        assert_eq!(TrackType::Event.to_string(), "[Zap] Event");
    }

    // Color tests - verify color RGB values
    #[test]
    fn track_type_camera_color_rgb() {
        let color = TrackType::Camera.color();
        // Cornflower blue: (100, 149, 237)
        assert_eq!(color.r(), 100);
        assert_eq!(color.g(), 149);
        assert_eq!(color.b(), 237);
    }

    #[test]
    fn track_type_animation_color_rgb() {
        let color = TrackType::Animation.color();
        // Light green: (144, 238, 144)
        assert_eq!(color.r(), 144);
        assert_eq!(color.g(), 238);
        assert_eq!(color.b(), 144);
    }

    #[test]
    fn track_type_audio_color_rgb() {
        let color = TrackType::Audio.color();
        // Orange: (255, 165, 0)
        assert_eq!(color.r(), 255);
        assert_eq!(color.g(), 165);
        assert_eq!(color.b(), 0);
    }

    #[test]
    fn track_type_fx_color_rgb() {
        let color = TrackType::Fx.color();
        // Medium orchid: (186, 85, 211)
        assert_eq!(color.r(), 186);
        assert_eq!(color.g(), 85);
        assert_eq!(color.b(), 211);
    }

    #[test]
    fn track_type_dialogue_color_rgb() {
        let color = TrackType::Dialogue.color();
        // Gold: (255, 215, 0)
        assert_eq!(color.r(), 255);
        assert_eq!(color.g(), 215);
        assert_eq!(color.b(), 0);
    }

    #[test]
    fn track_type_event_color_rgb() {
        let color = TrackType::Event.color();
        // Crimson: (220, 20, 60)
        assert_eq!(color.r(), 220);
        assert_eq!(color.g(), 20);
        assert_eq!(color.b(), 60);
    }

    // all() tests
    #[test]
    fn track_type_all_count() {
        assert_eq!(TrackType::all().len(), 6);
    }

    #[test]
    fn track_type_all_contains_camera() {
        assert!(TrackType::all().contains(&TrackType::Camera));
    }

    #[test]
    fn track_type_all_contains_animation() {
        assert!(TrackType::all().contains(&TrackType::Animation));
    }

    #[test]
    fn track_type_all_contains_audio() {
        assert!(TrackType::all().contains(&TrackType::Audio));
    }

    #[test]
    fn track_type_all_contains_fx() {
        assert!(TrackType::all().contains(&TrackType::Fx));
    }

    #[test]
    fn track_type_all_contains_dialogue() {
        assert!(TrackType::all().contains(&TrackType::Dialogue));
    }

    #[test]
    fn track_type_all_contains_event() {
        assert!(TrackType::all().contains(&TrackType::Event));
    }

    // Default test
    #[test]
    fn track_type_default_is_camera() {
        assert_eq!(TrackType::default(), TrackType::Camera);
    }

    // Uniqueness tests
    #[test]
    fn track_type_names_are_unique() {
        let names: Vec<_> = TrackType::all().iter().map(|t| t.name()).collect();
        let mut unique = names.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(names.len(), unique.len());
    }

    #[test]
    fn track_type_icons_are_unique() {
        let icons: Vec<_> = TrackType::all().iter().map(|t| t.icon()).collect();
        let mut unique = icons.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(icons.len(), unique.len());
    }
}

// ============================================================================
// CameraInterpolation Tests - 5 variants with is_smooth()
// ============================================================================

mod camera_interpolation_tests {
    use super::*;

    // Name tests
    #[test]
    fn camera_interp_linear_name() {
        assert_eq!(CameraInterpolation::Linear.name(), "Linear");
    }

    #[test]
    fn camera_interp_catmull_rom_name() {
        assert_eq!(CameraInterpolation::CatmullRom.name(), "Catmull-Rom");
    }

    #[test]
    fn camera_interp_bezier_name() {
        assert_eq!(CameraInterpolation::Bezier.name(), "Bezier");
    }

    #[test]
    fn camera_interp_hermite_name() {
        assert_eq!(CameraInterpolation::Hermite.name(), "Hermite");
    }

    #[test]
    fn camera_interp_step_name() {
        assert_eq!(CameraInterpolation::Step.name(), "Step");
    }

    // is_smooth() tests - true path (4 variants)
    #[test]
    fn camera_interp_linear_is_smooth() {
        assert!(CameraInterpolation::Linear.is_smooth());
    }

    #[test]
    fn camera_interp_catmull_rom_is_smooth() {
        assert!(CameraInterpolation::CatmullRom.is_smooth());
    }

    #[test]
    fn camera_interp_bezier_is_smooth() {
        assert!(CameraInterpolation::Bezier.is_smooth());
    }

    #[test]
    fn camera_interp_hermite_is_smooth() {
        assert!(CameraInterpolation::Hermite.is_smooth());
    }

    // is_smooth() tests - false path (1 variant)
    #[test]
    fn camera_interp_step_is_not_smooth() {
        assert!(!CameraInterpolation::Step.is_smooth());
    }

    // Count verification for is_smooth paths
    #[test]
    fn camera_interp_smooth_count() {
        let smooth_count = CameraInterpolation::all()
            .iter()
            .filter(|i| i.is_smooth())
            .count();
        assert_eq!(smooth_count, 4);
    }

    #[test]
    fn camera_interp_not_smooth_count() {
        let not_smooth_count = CameraInterpolation::all()
            .iter()
            .filter(|i| !i.is_smooth())
            .count();
        assert_eq!(not_smooth_count, 1);
    }

    // Display tests
    #[test]
    fn camera_interp_linear_display() {
        assert_eq!(CameraInterpolation::Linear.to_string(), "Linear");
    }

    #[test]
    fn camera_interp_step_display() {
        assert_eq!(CameraInterpolation::Step.to_string(), "Step");
    }

    // all() tests
    #[test]
    fn camera_interp_all_count() {
        assert_eq!(CameraInterpolation::all().len(), 5);
    }

    #[test]
    fn camera_interp_all_contains_linear() {
        assert!(CameraInterpolation::all().contains(&CameraInterpolation::Linear));
    }

    #[test]
    fn camera_interp_all_contains_step() {
        assert!(CameraInterpolation::all().contains(&CameraInterpolation::Step));
    }

    // Default test
    #[test]
    fn camera_interp_default_is_linear() {
        assert_eq!(CameraInterpolation::default(), CameraInterpolation::Linear);
    }
}

// ============================================================================
// PlaybackState Tests - 4 variants with is_running()
// ============================================================================

mod playback_state_tests {
    use super::*;

    // Name tests
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
    fn playback_state_recording_name() {
        assert_eq!(PlaybackState::Recording.name(), "Recording");
    }

    // Icon tests
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
    fn playback_state_recording_icon() {
        assert_eq!(PlaybackState::Recording.icon(), "⏺️");
    }

    // is_running() tests - true path (2 variants)
    #[test]
    fn playback_state_playing_is_running() {
        assert!(PlaybackState::Playing.is_running());
    }

    #[test]
    fn playback_state_recording_is_running() {
        assert!(PlaybackState::Recording.is_running());
    }

    // is_running() tests - false path (2 variants)
    #[test]
    fn playback_state_stopped_is_not_running() {
        assert!(!PlaybackState::Stopped.is_running());
    }

    #[test]
    fn playback_state_paused_is_not_running() {
        assert!(!PlaybackState::Paused.is_running());
    }

    // Count verification
    #[test]
    fn playback_state_running_count() {
        let states = [
            PlaybackState::Stopped,
            PlaybackState::Playing,
            PlaybackState::Paused,
            PlaybackState::Recording,
        ];
        let running_count = states.iter().filter(|s| s.is_running()).count();
        assert_eq!(running_count, 2);
    }

    #[test]
    fn playback_state_not_running_count() {
        let states = [
            PlaybackState::Stopped,
            PlaybackState::Playing,
            PlaybackState::Paused,
            PlaybackState::Recording,
        ];
        let not_running_count = states.iter().filter(|s| !s.is_running()).count();
        assert_eq!(not_running_count, 2);
    }

    // Display tests
    #[test]
    fn playback_state_stopped_display() {
        assert_eq!(PlaybackState::Stopped.to_string(), "[] Stopped");
    }

    #[test]
    fn playback_state_playing_display() {
        assert_eq!(PlaybackState::Playing.to_string(), "> Playing");
    }

    #[test]
    fn playback_state_paused_display() {
        assert_eq!(PlaybackState::Paused.to_string(), "|| Paused");
    }

    #[test]
    fn playback_state_recording_display() {
        assert_eq!(PlaybackState::Recording.to_string(), "⏺️ Recording");
    }

    // Default test
    #[test]
    fn playback_state_default_is_stopped() {
        assert_eq!(PlaybackState::default(), PlaybackState::Stopped);
    }
}

// ============================================================================
// PlaybackSpeed Tests - 5 variants with multiplier()
// ============================================================================

mod playback_speed_tests {
    use super::*;

    // multiplier() tests - verify exact values
    #[test]
    fn playback_speed_quarter_multiplier() {
        assert_eq!(PlaybackSpeed::Quarter.multiplier(), 0.25);
    }

    #[test]
    fn playback_speed_half_multiplier() {
        assert_eq!(PlaybackSpeed::Half.multiplier(), 0.5);
    }

    #[test]
    fn playback_speed_normal_multiplier() {
        assert_eq!(PlaybackSpeed::Normal.multiplier(), 1.0);
    }

    #[test]
    fn playback_speed_double_multiplier() {
        assert_eq!(PlaybackSpeed::Double.multiplier(), 2.0);
    }

    #[test]
    fn playback_speed_quadruple_multiplier() {
        assert_eq!(PlaybackSpeed::Quadruple.multiplier(), 4.0);
    }

    // Boundary tests - ordering verification
    #[test]
    fn playback_speed_quarter_is_slowest() {
        let quarter = PlaybackSpeed::Quarter.multiplier();
        for speed in PlaybackSpeed::all() {
            assert!(quarter <= speed.multiplier());
        }
    }

    #[test]
    fn playback_speed_quadruple_is_fastest() {
        let quadruple = PlaybackSpeed::Quadruple.multiplier();
        for speed in PlaybackSpeed::all() {
            assert!(quadruple >= speed.multiplier());
        }
    }

    #[test]
    fn playback_speed_multiplier_ordering() {
        let quarter = PlaybackSpeed::Quarter.multiplier();
        let half = PlaybackSpeed::Half.multiplier();
        let normal = PlaybackSpeed::Normal.multiplier();
        let double = PlaybackSpeed::Double.multiplier();
        let quadruple = PlaybackSpeed::Quadruple.multiplier();

        assert!(quarter < half);
        assert!(half < normal);
        assert!(normal < double);
        assert!(double < quadruple);
    }

    // display() tests
    #[test]
    fn playback_speed_quarter_display() {
        assert_eq!(PlaybackSpeed::Quarter.display(), "0.25×");
    }

    #[test]
    fn playback_speed_half_display() {
        assert_eq!(PlaybackSpeed::Half.display(), "0.5×");
    }

    #[test]
    fn playback_speed_normal_display() {
        assert_eq!(PlaybackSpeed::Normal.display(), "1×");
    }

    #[test]
    fn playback_speed_double_display() {
        assert_eq!(PlaybackSpeed::Double.display(), "2×");
    }

    #[test]
    fn playback_speed_quadruple_display() {
        assert_eq!(PlaybackSpeed::Quadruple.display(), "4×");
    }

    // Display trait tests
    #[test]
    fn playback_speed_display_trait_quarter() {
        assert_eq!(PlaybackSpeed::Quarter.to_string(), "0.25×");
    }

    #[test]
    fn playback_speed_display_trait_normal() {
        assert_eq!(PlaybackSpeed::Normal.to_string(), "1×");
    }

    // all() tests
    #[test]
    fn playback_speed_all_count() {
        assert_eq!(PlaybackSpeed::all().len(), 5);
    }

    #[test]
    fn playback_speed_all_contains_quarter() {
        assert!(PlaybackSpeed::all().contains(&PlaybackSpeed::Quarter));
    }

    #[test]
    fn playback_speed_all_contains_normal() {
        assert!(PlaybackSpeed::all().contains(&PlaybackSpeed::Normal));
    }

    #[test]
    fn playback_speed_all_contains_quadruple() {
        assert!(PlaybackSpeed::all().contains(&PlaybackSpeed::Quadruple));
    }

    // Default test
    #[test]
    fn playback_speed_default_is_normal() {
        assert_eq!(PlaybackSpeed::default(), PlaybackSpeed::Normal);
    }

    // Slow motion / fast forward classification
    #[test]
    fn playback_speed_slow_motion_count() {
        let slow_count = PlaybackSpeed::all()
            .iter()
            .filter(|s| s.multiplier() < 1.0)
            .count();
        assert_eq!(slow_count, 2);
    }

    #[test]
    fn playback_speed_fast_forward_count() {
        let fast_count = PlaybackSpeed::all()
            .iter()
            .filter(|s| s.multiplier() > 1.0)
            .count();
        assert_eq!(fast_count, 2);
    }

    #[test]
    fn playback_speed_normal_count() {
        let normal_count = PlaybackSpeed::all()
            .iter()
            .filter(|s| s.multiplier() == 1.0)
            .count();
        assert_eq!(normal_count, 1);
    }
}

// ============================================================================
// CinematicsTab Tests - 6 variants
// ============================================================================

mod cinematics_tab_tests {
    use super::*;

    // Name tests
    #[test]
    fn cinematics_tab_timeline_name() {
        assert_eq!(CinematicsTab::Timeline.name(), "Timeline");
    }

    #[test]
    fn cinematics_tab_camera_name() {
        assert_eq!(CinematicsTab::Camera.name(), "Camera");
    }

    #[test]
    fn cinematics_tab_tracks_name() {
        assert_eq!(CinematicsTab::Tracks.name(), "Tracks");
    }

    #[test]
    fn cinematics_tab_clips_name() {
        assert_eq!(CinematicsTab::Clips.name(), "Clips");
    }

    #[test]
    fn cinematics_tab_preview_name() {
        assert_eq!(CinematicsTab::Preview.name(), "Preview");
    }

    #[test]
    fn cinematics_tab_export_name() {
        assert_eq!(CinematicsTab::Export.name(), "Export");
    }

    // Icon tests
    #[test]
    fn cinematics_tab_timeline_icon() {
        assert_eq!(CinematicsTab::Timeline.icon(), "📅");
    }

    #[test]
    fn cinematics_tab_camera_icon() {
        assert_eq!(CinematicsTab::Camera.icon(), "📷");
    }

    #[test]
    fn cinematics_tab_tracks_icon() {
        assert_eq!(CinematicsTab::Tracks.icon(), "[Anim]");
    }

    #[test]
    fn cinematics_tab_clips_icon() {
        assert_eq!(CinematicsTab::Clips.icon(), "🎞️");
    }

    #[test]
    fn cinematics_tab_preview_icon() {
        assert_eq!(CinematicsTab::Preview.icon(), "👁️");
    }

    #[test]
    fn cinematics_tab_export_icon() {
        assert_eq!(CinematicsTab::Export.icon(), "[Save]");
    }

    // Display tests
    #[test]
    fn cinematics_tab_timeline_display() {
        assert_eq!(CinematicsTab::Timeline.to_string(), "📅 Timeline");
    }

    #[test]
    fn cinematics_tab_camera_display() {
        assert_eq!(CinematicsTab::Camera.to_string(), "📷 Camera");
    }

    #[test]
    fn cinematics_tab_tracks_display() {
        assert_eq!(CinematicsTab::Tracks.to_string(), "[Anim] Tracks");
    }

    #[test]
    fn cinematics_tab_clips_display() {
        assert_eq!(CinematicsTab::Clips.to_string(), "🎞️ Clips");
    }

    #[test]
    fn cinematics_tab_preview_display() {
        assert_eq!(CinematicsTab::Preview.to_string(), "👁️ Preview");
    }

    #[test]
    fn cinematics_tab_export_display() {
        assert_eq!(CinematicsTab::Export.to_string(), "[Save] Export");
    }

    // all() tests
    #[test]
    fn cinematics_tab_all_count() {
        assert_eq!(CinematicsTab::all().len(), 6);
    }

    #[test]
    fn cinematics_tab_all_contains_timeline() {
        assert!(CinematicsTab::all().contains(&CinematicsTab::Timeline));
    }

    #[test]
    fn cinematics_tab_all_contains_camera() {
        assert!(CinematicsTab::all().contains(&CinematicsTab::Camera));
    }

    #[test]
    fn cinematics_tab_all_contains_tracks() {
        assert!(CinematicsTab::all().contains(&CinematicsTab::Tracks));
    }

    #[test]
    fn cinematics_tab_all_contains_clips() {
        assert!(CinematicsTab::all().contains(&CinematicsTab::Clips));
    }

    #[test]
    fn cinematics_tab_all_contains_preview() {
        assert!(CinematicsTab::all().contains(&CinematicsTab::Preview));
    }

    #[test]
    fn cinematics_tab_all_contains_export() {
        assert!(CinematicsTab::all().contains(&CinematicsTab::Export));
    }

    // Default test
    #[test]
    fn cinematics_tab_default_is_timeline() {
        assert_eq!(CinematicsTab::default(), CinematicsTab::Timeline);
    }

    // Uniqueness tests
    #[test]
    fn cinematics_tab_names_are_unique() {
        let names: Vec<_> = CinematicsTab::all().iter().map(|t| t.name()).collect();
        let mut unique = names.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(names.len(), unique.len());
    }

    #[test]
    fn cinematics_tab_icons_are_unique() {
        let icons: Vec<_> = CinematicsTab::all().iter().map(|t| t.icon()).collect();
        let mut unique = icons.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(icons.len(), unique.len());
    }
}

// ============================================================================
// Equality and Hash Tests - verify derive traits work correctly
// ============================================================================

mod equality_tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn track_type_eq() {
        assert_eq!(TrackType::Camera, TrackType::Camera);
        assert_ne!(TrackType::Camera, TrackType::Audio);
    }

    #[test]
    fn track_type_hash() {
        let mut set = HashSet::new();
        set.insert(TrackType::Camera);
        set.insert(TrackType::Audio);
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn camera_interp_eq() {
        assert_eq!(CameraInterpolation::Linear, CameraInterpolation::Linear);
        assert_ne!(CameraInterpolation::Linear, CameraInterpolation::Step);
    }

    #[test]
    fn camera_interp_hash() {
        let mut set = HashSet::new();
        set.insert(CameraInterpolation::Linear);
        set.insert(CameraInterpolation::Bezier);
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn playback_state_eq() {
        assert_eq!(PlaybackState::Playing, PlaybackState::Playing);
        assert_ne!(PlaybackState::Playing, PlaybackState::Paused);
    }

    #[test]
    fn playback_state_hash() {
        let mut set = HashSet::new();
        set.insert(PlaybackState::Playing);
        set.insert(PlaybackState::Paused);
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn cinematics_tab_eq() {
        assert_eq!(CinematicsTab::Timeline, CinematicsTab::Timeline);
        assert_ne!(CinematicsTab::Timeline, CinematicsTab::Export);
    }

    #[test]
    fn cinematics_tab_hash() {
        let mut set = HashSet::new();
        set.insert(CinematicsTab::Timeline);
        set.insert(CinematicsTab::Export);
        assert_eq!(set.len(), 2);
    }
}

// ============================================================================
// Clone and Debug Tests - verify derive traits work correctly
// ============================================================================

mod clone_debug_tests {
    use super::*;

    #[test]
    fn track_type_clone() {
        let t = TrackType::Camera;
        let cloned = t.clone();
        assert_eq!(t, cloned);
    }

    #[test]
    fn track_type_debug() {
        let debug = format!("{:?}", TrackType::Camera);
        assert!(debug.contains("Camera"));
    }

    #[test]
    fn camera_interp_clone() {
        let i = CameraInterpolation::Bezier;
        let cloned = i.clone();
        assert_eq!(i, cloned);
    }

    #[test]
    fn camera_interp_debug() {
        let debug = format!("{:?}", CameraInterpolation::Bezier);
        assert!(debug.contains("Bezier"));
    }

    #[test]
    fn playback_state_clone() {
        let s = PlaybackState::Playing;
        let cloned = s.clone();
        assert_eq!(s, cloned);
    }

    #[test]
    fn playback_state_debug() {
        let debug = format!("{:?}", PlaybackState::Playing);
        assert!(debug.contains("Playing"));
    }

    #[test]
    fn playback_speed_clone() {
        let s = PlaybackSpeed::Double;
        let cloned = s.clone();
        assert_eq!(s, cloned);
    }

    #[test]
    fn playback_speed_debug() {
        let debug = format!("{:?}", PlaybackSpeed::Double);
        assert!(debug.contains("Double"));
    }

    #[test]
    fn cinematics_tab_clone() {
        let t = CinematicsTab::Preview;
        let cloned = t.clone();
        assert_eq!(t, cloned);
    }

    #[test]
    fn cinematics_tab_debug() {
        let debug = format!("{:?}", CinematicsTab::Preview);
        assert!(debug.contains("Preview"));
    }
}

// ============================================================================
// Comprehensive Boolean Path Tests
// ============================================================================

mod boolean_path_tests {
    use super::*;

    // Verify all is_smooth paths are tested
    #[test]
    fn camera_interp_all_smooth_paths_tested() {
        // 4 true paths
        let smooth_true = [
            CameraInterpolation::Linear,
            CameraInterpolation::CatmullRom,
            CameraInterpolation::Bezier,
            CameraInterpolation::Hermite,
        ];
        for interp in smooth_true {
            assert!(interp.is_smooth(), "{:?} should be smooth", interp);
        }

        // 1 false path
        assert!(
            !CameraInterpolation::Step.is_smooth(),
            "Step should not be smooth"
        );
    }

    // Verify all is_running paths are tested
    #[test]
    fn playback_state_all_running_paths_tested() {
        // 2 true paths
        let running_true = [PlaybackState::Playing, PlaybackState::Recording];
        for state in running_true {
            assert!(state.is_running(), "{:?} should be running", state);
        }

        // 2 false paths
        let running_false = [PlaybackState::Stopped, PlaybackState::Paused];
        for state in running_false {
            assert!(!state.is_running(), "{:?} should not be running", state);
        }
    }
}

// ============================================================================
// Multiplier Value Boundary Tests
// ============================================================================

mod multiplier_boundary_tests {
    use super::*;

    #[test]
    fn multiplier_positive_for_all() {
        for speed in PlaybackSpeed::all() {
            assert!(
                speed.multiplier() > 0.0,
                "{:?} should have positive multiplier",
                speed
            );
        }
    }

    #[test]
    fn multiplier_quarter_is_exactly_0_25() {
        let m = PlaybackSpeed::Quarter.multiplier();
        assert!((m - 0.25).abs() < f32::EPSILON);
    }

    #[test]
    fn multiplier_half_is_exactly_0_5() {
        let m = PlaybackSpeed::Half.multiplier();
        assert!((m - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn multiplier_normal_is_exactly_1_0() {
        let m = PlaybackSpeed::Normal.multiplier();
        assert!((m - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn multiplier_double_is_exactly_2_0() {
        let m = PlaybackSpeed::Double.multiplier();
        assert!((m - 2.0).abs() < f32::EPSILON);
    }

    #[test]
    fn multiplier_quadruple_is_exactly_4_0() {
        let m = PlaybackSpeed::Quadruple.multiplier();
        assert!((m - 4.0).abs() < f32::EPSILON);
    }
}

// ============================================================================
// Color Component Boundary Tests
// ============================================================================

mod color_boundary_tests {
    use super::*;

    #[test]
    fn track_type_colors_valid_rgb() {
        for track_type in TrackType::all() {
            let color = track_type.color();
            // RGB values should be 0-255 (implicit in u8 type)
            let _ = color.r();
            let _ = color.g();
            let _ = color.b();
            // Alpha should be 255 (opaque) for named colors
            assert_eq!(color.a(), 255, "{:?} should have full alpha", track_type);
        }
    }

    #[test]
    fn camera_color_is_cornflower_blue() {
        let color = TrackType::Camera.color();
        assert_eq!((color.r(), color.g(), color.b()), (100, 149, 237));
    }

    #[test]
    fn animation_color_is_light_green() {
        let color = TrackType::Animation.color();
        assert_eq!((color.r(), color.g(), color.b()), (144, 238, 144));
    }

    #[test]
    fn audio_color_is_orange() {
        let color = TrackType::Audio.color();
        assert_eq!((color.r(), color.g(), color.b()), (255, 165, 0));
    }

    #[test]
    fn fx_color_is_medium_orchid() {
        let color = TrackType::Fx.color();
        assert_eq!((color.r(), color.g(), color.b()), (186, 85, 211));
    }

    #[test]
    fn dialogue_color_is_gold() {
        let color = TrackType::Dialogue.color();
        assert_eq!((color.r(), color.g(), color.b()), (255, 215, 0));
    }

    #[test]
    fn event_color_is_crimson() {
        let color = TrackType::Event.color();
        assert_eq!((color.r(), color.g(), color.b()), (220, 20, 60));
    }
}
