//! Comprehensive file-based audio API tests
//!
//! These tests exercise all file-based audio APIs that were previously
//! untestable without audio assets. Uses generated test assets from
//! test_asset_generator.rs.

mod test_asset_generator;

use anyhow::Result;
use astraweave_audio::{AudioEngine, ListenerPose, MusicTrack, PanMode};
use glam::vec3;
use serial_test::serial;
use std::thread;
use std::time::Duration;

/// Setup helper: Generate all test assets before running file-based tests
fn setup_test_assets() -> Result<()> {
    test_asset_generator::setup_all_test_assets()
}

/// Test 1: Play SFX from file (basic file loading)
#[test]
#[serial]
fn test_play_sfx_file_basic() -> Result<()> {
    setup_test_assets()?;

    let mut engine = AudioEngine::new()?;

    // Play short beep from file
    let result = engine.play_sfx_file(test_asset_generator::asset_path("test_beep_440hz.wav"));
    assert!(result.is_ok(), "Should play SFX file successfully");

    // Allow audio to start playing
    thread::sleep(Duration::from_millis(50));
    engine.tick(0.05);

    Ok(())
}

/// Test 2: Play multiple SFX files in sequence
#[test]
#[serial]
fn test_play_multiple_sfx_files() -> Result<()> {
    setup_test_assets()?;

    let mut engine = AudioEngine::new()?;

    // Play three different frequency beeps
    engine.play_sfx_file(test_asset_generator::asset_path("test_beep_200hz.wav"))?;
    thread::sleep(Duration::from_millis(100));

    engine.play_sfx_file(test_asset_generator::asset_path("test_beep_440hz.wav"))?;
    thread::sleep(Duration::from_millis(100));

    engine.play_sfx_file(test_asset_generator::asset_path("test_beep_1000hz.wav"))?;
    thread::sleep(Duration::from_millis(100));

    engine.tick(0.3);

    Ok(())
}

/// Test 3: Play voice file with automatic ducking
#[test]
#[serial]
fn test_play_voice_file_with_ducking() -> Result<()> {
    setup_test_assets()?;

    let mut engine = AudioEngine::new()?;

    // Play voice file (should duck music if playing)
    let result = engine.play_voice_file(
        test_asset_generator::asset_path("test_voice_short.wav"),
        None,
    );
    assert!(result.is_ok(), "Should play voice file successfully");

    // Tick to activate ducking
    engine.tick(0.016);

    // Wait for voice duration
    thread::sleep(Duration::from_millis(200));
    engine.tick(0.2);

    Ok(())
}

/// Test 4: Play voice file with explicit duration
#[test]
#[serial]
fn test_play_voice_file_explicit_duration() -> Result<()> {
    setup_test_assets()?;

    let mut engine = AudioEngine::new()?;

    // Play voice with explicit 2.5 second duration
    let result = engine.play_voice_file(
        test_asset_generator::asset_path("test_voice_medium.wav"),
        Some(2.5),
    );
    assert!(
        result.is_ok(),
        "Should play voice file with explicit duration"
    );

    engine.tick(0.016);

    Ok(())
}

/// Test 5: Play 3D spatial SFX from file
#[test]
#[serial]
fn test_play_sfx_3d_file() -> Result<()> {
    setup_test_assets()?;

    let mut engine = AudioEngine::new()?;
    let emitter_id = 1;
    let position = vec3(5.0, 0.0, 0.0); // 5 meters to the right

    // Play spatial SFX from file
    let result = engine.play_sfx_3d_file(
        emitter_id,
        test_asset_generator::asset_path("test_beep_440hz.wav"),
        position,
    );
    assert!(result.is_ok(), "Should play 3D SFX file successfully");

    engine.tick(0.016);

    Ok(())
}

/// Test 6: Multiple 3D sounds from files at different positions
#[test]
#[serial]
fn test_multiple_3d_files() -> Result<()> {
    setup_test_assets()?;

    let mut engine = AudioEngine::new()?;

    // Create 4 spatial emitters in cardinal directions
    let positions = [
        (1, vec3(5.0, 0.0, 0.0)),  // Right
        (2, vec3(-5.0, 0.0, 0.0)), // Left
        (3, vec3(0.0, 0.0, 5.0)),  // Forward
        (4, vec3(0.0, 0.0, -5.0)), // Behind
    ];

    let files = [
        test_asset_generator::asset_path("test_beep_200hz.wav"),
        test_asset_generator::asset_path("test_beep_440hz.wav"),
        test_asset_generator::asset_path("test_beep_1000hz.wav"),
        test_asset_generator::asset_path("test_beep_short.wav"),
    ];

    for ((id, pos), file) in positions.iter().zip(files.iter()) {
        let result = engine.play_sfx_3d_file(*id, file, *pos);
        assert!(result.is_ok(), "Should play 3D file at position {:?}", pos);
    }

    engine.tick(0.016);

    Ok(())
}

/// Test 7: Music playback from file (basic)
#[test]
#[serial]
fn test_play_music_file_basic() -> Result<()> {
    setup_test_assets()?;

    let mut engine = AudioEngine::new()?;

    let track = MusicTrack {
        path: test_asset_generator::asset_path("test_music_2sec.wav").to_string(),
        looped: false,
    };

    // Play music with instant crossfade (no previous track)
    let result = engine.play_music(track, 0.0);
    assert!(result.is_ok(), "Should play music file successfully");

    engine.tick(0.016);

    Ok(())
}

/// Test 8: Music playback with looping enabled
#[test]
#[serial]
fn test_play_music_looped() -> Result<()> {
    setup_test_assets()?;

    let mut engine = AudioEngine::new()?;

    let track = MusicTrack {
        path: test_asset_generator::asset_path("test_music_2sec.wav").to_string(),
        looped: true,
    };

    let result = engine.play_music(track, 0.0);
    assert!(result.is_ok(), "Should play looped music successfully");

    // Tick through several seconds to test looping
    for _ in 0..100 {
        engine.tick(0.016);
    }

    Ok(())
}

/// Test 9: Music crossfade between two tracks
#[test]
#[serial]
fn test_music_crossfade() -> Result<()> {
    setup_test_assets()?;

    let mut engine = AudioEngine::new()?;

    // Play first track
    let track1 = MusicTrack {
        path: test_asset_generator::asset_path("test_music_2sec.wav").to_string(),
        looped: true,
    };
    engine.play_music(track1, 0.0)?;

    // Wait a bit
    thread::sleep(Duration::from_millis(500));
    engine.tick(0.5);

    // Start crossfade to second track (1 second crossfade)
    let track2 = MusicTrack {
        path: test_asset_generator::asset_path("test_music_5sec.wav").to_string(),
        looped: true,
    };
    let result = engine.play_music(track2, 1.0);
    assert!(result.is_ok(), "Should start crossfade to new track");

    // Tick through crossfade duration
    for _ in 0..60 {
        engine.tick(0.016); // 60 ticks = ~1 second
    }

    Ok(())
}

/// Test 10: Fast crossfade (0.1 second)
#[test]
#[serial]
fn test_music_fast_crossfade() -> Result<()> {
    setup_test_assets()?;

    let mut engine = AudioEngine::new()?;

    // Play first track
    let track1 = MusicTrack {
        path: test_asset_generator::asset_path("test_music_2sec.wav").to_string(),
        looped: false,
    };
    engine.play_music(track1, 0.0)?;
    engine.tick(0.1);

    // Fast crossfade to second track
    let track2 = MusicTrack {
        path: test_asset_generator::asset_path("test_music_5sec.wav").to_string(),
        looped: false,
    };
    engine.play_music(track2, 0.1)?;

    // Tick through fast crossfade
    for _ in 0..10 {
        engine.tick(0.016);
    }

    Ok(())
}

/// Test 11: Stop music while playing
#[test]
#[serial]
fn test_stop_music_while_playing() -> Result<()> {
    setup_test_assets()?;

    let mut engine = AudioEngine::new()?;

    // Start music
    let track = MusicTrack {
        path: test_asset_generator::asset_path("test_music_5sec.wav").to_string(),
        looped: true,
    };
    engine.play_music(track, 0.0)?;
    engine.tick(0.1);

    // Stop music
    engine.stop_music();
    engine.tick(0.016);

    Ok(())
}

/// Test 12: Voice file playback during music (ducking test)
#[test]
#[serial]
fn test_voice_ducking_with_music() -> Result<()> {
    setup_test_assets()?;

    let mut engine = AudioEngine::new()?;

    // Start music
    let track = MusicTrack {
        path: test_asset_generator::asset_path("test_music_5sec.wav").to_string(),
        looped: true,
    };
    engine.play_music(track, 0.0)?;
    engine.tick(0.1);

    // Play voice (should duck music)
    engine.play_voice_file(
        test_asset_generator::asset_path("test_voice_short.wav"),
        None,
    )?;
    engine.tick(0.016);

    // Tick through voice duration + restoration time
    for _ in 0..200 {
        engine.tick(0.016); // ~3 seconds
    }

    Ok(())
}

/// Test 13: Multiple voice files in sequence (ducking restoration)
#[test]
#[serial]
fn test_multiple_voice_files_ducking() -> Result<()> {
    setup_test_assets()?;

    let mut engine = AudioEngine::new()?;

    // Start music
    let track = MusicTrack {
        path: test_asset_generator::asset_path("test_music_5sec.wav").to_string(),
        looped: true,
    };
    engine.play_music(track, 0.0)?;
    engine.tick(0.1);

    // Play three voice files in succession
    engine.play_voice_file(
        test_asset_generator::asset_path("test_voice_short.wav"),
        Some(1.0),
    )?;
    thread::sleep(Duration::from_millis(200));
    engine.tick(0.2);

    engine.play_voice_file(
        test_asset_generator::asset_path("test_voice_medium.wav"),
        Some(2.0),
    )?;
    thread::sleep(Duration::from_millis(300));
    engine.tick(0.3);

    engine.play_voice_file(
        test_asset_generator::asset_path("test_voice_long.wav"),
        Some(3.0),
    )?;
    thread::sleep(Duration::from_millis(400));
    engine.tick(0.4);

    // Allow ducking to restore
    for _ in 0..100 {
        engine.tick(0.016);
    }

    Ok(())
}

/// Test 14: 3D sound with listener movement (file-based)
#[test]
#[serial]
fn test_3d_file_with_listener_movement() -> Result<()> {
    setup_test_assets()?;

    let mut engine = AudioEngine::new()?;

    let emitter_id = 1;
    let sound_pos = vec3(10.0, 0.0, 0.0);

    // Play long 3D sound
    engine.play_sfx_3d_file(
        emitter_id,
        test_asset_generator::asset_path("test_beep_long.wav"),
        sound_pos,
    )?;

    // Move listener in a circle around the sound
    for i in 0..36 {
        let angle = (i as f32) * 10.0 * std::f32::consts::PI / 180.0;
        let listener_pos = vec3(angle.cos() * 5.0, 0.0, angle.sin() * 5.0);

        let pose = ListenerPose {
            position: listener_pos,
            forward: (sound_pos - listener_pos).normalize_or_zero(),
            up: vec3(0.0, 1.0, 0.0),
        };

        engine.update_listener(pose);
        engine.tick(0.016);
    }

    Ok(())
}

/// Test 15: Stress test - 20 file-based 3D sounds simultaneously
#[test]
#[serial]
fn test_stress_20_file_based_3d_sounds() -> Result<()> {
    setup_test_assets()?;

    let mut engine = AudioEngine::new()?;

    let files = [
        test_asset_generator::asset_path("test_beep_200hz.wav"),
        test_asset_generator::asset_path("test_beep_440hz.wav"),
        test_asset_generator::asset_path("test_beep_1000hz.wav"),
        test_asset_generator::asset_path("test_beep_short.wav"),
    ];

    // Create 20 emitters in a grid
    for i in 0..20 {
        let x = (i % 5) as f32 * 3.0;
        let z = (i / 5) as f32 * 3.0;
        let pos = vec3(x, 0.0, z);

        let file = files[i % files.len()];
        let result = engine.play_sfx_3d_file(i as u64, file, pos);
        assert!(result.is_ok(), "Should play 3D file {}", i);
    }

    // Tick through several frames
    for _ in 0..30 {
        engine.tick(0.016);
    }

    Ok(())
}

/// Test 16: Interleaved file and beep SFX
#[test]
#[serial]
fn test_interleaved_file_and_beep_sfx() -> Result<()> {
    setup_test_assets()?;

    let mut engine = AudioEngine::new()?;

    for i in 0..10 {
        if i % 2 == 0 {
            // File-based SFX
            engine.play_sfx_file(test_asset_generator::asset_path("test_beep_short.wav"))?;
        } else {
            // Synthesized beep
            engine.play_sfx_beep(880.0, 0.1, 0.5);
        }

        thread::sleep(Duration::from_millis(50));
        engine.tick(0.05);
    }

    Ok(())
}

/// Test 17: Volume control with file-based audio
#[test]
#[serial]
fn test_volume_control_with_files() -> Result<()> {
    setup_test_assets()?;

    let mut engine = AudioEngine::new()?;

    // Play file at full volume
    engine.set_master_volume(1.0);
    engine.play_sfx_file(test_asset_generator::asset_path("test_beep_440hz.wav"))?;
    engine.tick(0.1);

    // Play file at half volume
    engine.set_master_volume(0.5);
    engine.play_sfx_file(test_asset_generator::asset_path("test_beep_440hz.wav"))?;
    engine.tick(0.1);

    // Play file at minimum volume
    engine.set_master_volume(0.0);
    engine.play_sfx_file(test_asset_generator::asset_path("test_beep_440hz.wav"))?;
    engine.tick(0.1);

    Ok(())
}

/// Test 18: Pan mode with file-based 3D audio
#[test]
#[serial]
fn test_pan_modes_with_files() -> Result<()> {
    setup_test_assets()?;

    let mut engine = AudioEngine::new()?;
    let emitter_id = 1;
    let pos = vec3(5.0, 0.0, 0.0);

    // StereoAngle mode
    engine.set_pan_mode(PanMode::StereoAngle);
    engine.play_sfx_3d_file(
        emitter_id,
        test_asset_generator::asset_path("test_beep_440hz.wav"),
        pos,
    )?;
    engine.tick(0.2);

    // None mode
    engine.set_pan_mode(PanMode::None);
    engine.play_sfx_3d_file(
        emitter_id + 1,
        test_asset_generator::asset_path("test_beep_1000hz.wav"),
        pos,
    )?;
    engine.tick(0.2);

    Ok(())
}

/// Test 19: Long-running music with periodic ticks
#[test]
#[serial]
fn test_long_music_playback() -> Result<()> {
    setup_test_assets()?;

    let mut engine = AudioEngine::new()?;

    let track = MusicTrack {
        path: test_asset_generator::asset_path("test_music_5sec.wav").to_string(),
        looped: true,
    };
    engine.play_music(track, 0.0)?;

    // Simulate 10 seconds of gameplay
    for _ in 0..600 {
        engine.tick(0.016); // 60 FPS
    }

    Ok(())
}

/// Test 20: Rapid music track changes (stress test crossfading)
#[test]
#[serial]
fn test_rapid_music_changes() -> Result<()> {
    setup_test_assets()?;

    let mut engine = AudioEngine::new()?;

    let tracks = [
        test_asset_generator::asset_path("test_music_2sec.wav"),
        test_asset_generator::asset_path("test_music_5sec.wav"),
    ];

    // Change tracks 5 times with 0.5 second crossfades
    for i in 0..5 {
        let track = MusicTrack {
            path: tracks[i % tracks.len()].to_string(),
            looped: false,
        };
        engine.play_music(track, 0.5)?;

        // Wait half the crossfade time before switching again
        thread::sleep(Duration::from_millis(250));
        for _ in 0..15 {
            engine.tick(0.016);
        }
    }

    Ok(())
}

/// Test 21: Missing file error handling
#[test]
#[serial]
fn test_missing_file_error() -> Result<()> {
    let mut engine = AudioEngine::new()?;

    // Attempt to play non-existent file
    let result = engine.play_sfx_file(test_asset_generator::asset_path("nonexistent_file.wav"));
    assert!(result.is_err(), "Should return error for missing file");

    Ok(())
}

/// Test 22: Invalid file format error handling
#[test]
#[serial]
fn test_invalid_file_format() -> Result<()> {
    // Create a text file pretending to be WAV
    std::fs::create_dir_all(test_asset_generator::assets_root_str())?;
    std::fs::write(
        test_asset_generator::asset_path("invalid.wav"),
        "This is not a WAV file",
    )?;

    let mut engine = AudioEngine::new()?;

    // Attempt to play invalid file
    let result = engine.play_sfx_file(test_asset_generator::asset_path("invalid.wav"));
    assert!(
        result.is_err(),
        "Should return error for invalid file format"
    );

    // Cleanup
    std::fs::remove_file(test_asset_generator::asset_path("invalid.wav"))?;

    Ok(())
}

/// Test 23: Emitter reuse with file-based 3D audio
#[test]
#[serial]
fn test_emitter_reuse_with_files() -> Result<()> {
    setup_test_assets()?;

    let mut engine = AudioEngine::new()?;
    let emitter_id = 42;

    // Play at position 1
    engine.play_sfx_3d_file(
        emitter_id,
        test_asset_generator::asset_path("test_beep_200hz.wav"),
        vec3(5.0, 0.0, 0.0),
    )?;
    thread::sleep(Duration::from_millis(100));
    engine.tick(0.1);

    // Reuse same emitter at position 2
    engine.play_sfx_3d_file(
        emitter_id,
        test_asset_generator::asset_path("test_beep_440hz.wav"),
        vec3(-5.0, 0.0, 0.0),
    )?;
    thread::sleep(Duration::from_millis(100));
    engine.tick(0.1);

    // Reuse again at position 3
    engine.play_sfx_3d_file(
        emitter_id,
        test_asset_generator::asset_path("test_beep_1000hz.wav"),
        vec3(0.0, 5.0, 0.0),
    )?;
    thread::sleep(Duration::from_millis(100));
    engine.tick(0.1);

    Ok(())
}

/// Test 24: Distance attenuation with file-based audio
#[test]
#[serial]
fn test_distance_attenuation_files() -> Result<()> {
    setup_test_assets()?;

    let mut engine = AudioEngine::new()?;

    // Near sound
    engine.play_sfx_3d_file(
        1,
        test_asset_generator::asset_path("test_beep_440hz.wav"),
        vec3(1.0, 0.0, 0.0),
    )?;

    // Medium distance
    engine.play_sfx_3d_file(
        2,
        test_asset_generator::asset_path("test_beep_440hz.wav"),
        vec3(10.0, 0.0, 0.0),
    )?;

    // Far sound
    engine.play_sfx_3d_file(
        3,
        test_asset_generator::asset_path("test_beep_440hz.wav"),
        vec3(100.0, 0.0, 0.0),
    )?;

    engine.tick(0.1);

    Ok(())
}

/// Test 25: Comprehensive audio pipeline (music + voice + 3D SFX from files)
#[test]
#[serial]
fn test_comprehensive_file_based_pipeline() -> Result<()> {
    setup_test_assets()?;

    let mut engine = AudioEngine::new()?;

    // 1. Start background music
    let music = MusicTrack {
        path: test_asset_generator::asset_path("test_music_5sec.wav").to_string(),
        looped: true,
    };
    engine.play_music(music, 0.0)?;
    thread::sleep(Duration::from_millis(200));
    engine.tick(0.2);

    // 2. Play voice over (with ducking)
    engine.play_voice_file(
        test_asset_generator::asset_path("test_voice_short.wav"),
        Some(1.0),
    )?;
    thread::sleep(Duration::from_millis(300));
    engine.tick(0.3);

    // 3. Play 2D SFX
    engine.play_sfx_file(test_asset_generator::asset_path("test_beep_440hz.wav"))?;
    thread::sleep(Duration::from_millis(100));
    engine.tick(0.1);

    // 4. Play 3D spatial SFX
    engine.play_sfx_3d_file(
        1,
        test_asset_generator::asset_path("test_beep_1000hz.wav"),
        vec3(5.0, 0.0, 0.0),
    )?;
    engine.play_sfx_3d_file(
        2,
        test_asset_generator::asset_path("test_beep_200hz.wav"),
        vec3(-5.0, 0.0, 0.0),
    )?;

    // 5. Update listener position
    let pose = ListenerPose {
        position: vec3(0.0, 1.0, 0.0),
        forward: vec3(1.0, 0.0, 0.0),
        up: vec3(0.0, 1.0, 0.0),
    };
    engine.update_listener(pose);

    // 6. Tick through several frames
    for _ in 0..60 {
        engine.tick(0.016);
    }

    // 7. Stop music
    engine.stop_music();
    engine.tick(0.1);

    Ok(())
}
