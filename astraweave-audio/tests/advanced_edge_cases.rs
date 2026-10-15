//! Advanced edge case tests for astraweave-audio (push to 85%+)
//!
//! This test suite targets remaining uncovered lines identified in tarpaulin:
//! - dialogue_runtime.rs: File path edge cases (14 untested lines)
//! - engine.rs: Volume calculation edge cases, crossfade boundaries (24 untested lines)
//! - voice.rs: Load function edge case (1 untested line)
//!
//! Target: 85%+ coverage (155/182 lines)

use anyhow::Result;
use astraweave_audio::{AudioEngine, DialoguePlayer, MusicTrack};
use astraweave_gameplay::dialogue::{Dialogue, DialogueState, Line, Node};
use glam::Vec3;
use serial_test::serial;
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use std::thread;
use std::time::Duration;

#[path = "test_asset_generator.rs"]
mod test_asset_generator;

// ============================================================================
// Advanced Edge Case 1: Dialogue Path Construction with Special Characters
// ============================================================================

#[test]
#[serial]
fn test_dialogue_file_path_with_spaces() -> Result<()> {
    // Setup
    fs::create_dir_all("tests/assets/advanced/speakers/space name")?;
    test_asset_generator::generate_test_voice(
        "tests/assets/advanced/speakers/space name/test voice.wav",
        1.0,
        22050,
    )?;

    let voice_bank_toml = r#"
[speakers.space_speaker]
folder = "tests/assets/advanced/speakers/space name"
files = ["test voice.wav"]
"#;

    let toml_path = "tests/assets/advanced/space_voice_bank.toml";
    fs::create_dir_all("tests/assets/advanced")?;
    let mut file = File::create(toml_path)?;
    file.write_all(voice_bank_toml.as_bytes())?;
    drop(file);

    let bank = astraweave_audio::load_voice_bank(toml_path)?;

    let dialogue = Dialogue {
        id: "space_test".into(),
        start: "n0".into(),
        nodes: vec![Node {
            id: "n0".into(),
            line: Some(Line {
                speaker: "space_speaker".into(),
                text: "Testing spaces in path".into(),
                set_vars: vec![],
            }),
            choices: vec![],
            end: true,
        }],
    };
    let state = DialogueState::new(&dialogue);

    let mut audio_engine = AudioEngine::new()?;

    {
        let mut player = DialoguePlayer {
            audio: &mut audio_engine,
            bank: &bank,
            overrides: None,
            tts: None,
            subtitle_out: None,
        };

        let result = player.speak_current(&dialogue, &state)?;
        assert!(result, "Should handle file paths with spaces");
    }

    audio_engine.tick(0.05);
    thread::sleep(Duration::from_millis(50));

    // Cleanup
    let _ = fs::remove_dir_all("tests/assets/advanced");

    Ok(())
}

// ============================================================================
// Advanced Edge Case 2: Voice Ducking with Zero/Negative Timer
// ============================================================================

#[test]
#[serial]
fn test_voice_ducking_edge_cases() -> Result<()> {
    let mut engine = AudioEngine::new()?;

    // Generate test voice file
    test_asset_generator::generate_test_voice("tests/assets/test_voice_duck.wav", 0.05, 22050)?;

    // Case 1: Very short voice (should clamp to 0.1s minimum)
    engine.play_voice_file("tests/assets/test_voice_duck.wav", Some(0.01))?;
    engine.tick(0.02);

    // Case 2: Negative duration (should clamp to 0.1s minimum)
    engine.play_voice_file("tests/assets/test_voice_duck.wav", Some(-1.0))?;
    engine.tick(0.02);

    // Case 3: Very long duration (should clamp to 30s maximum)
    engine.play_voice_file("tests/assets/test_voice_duck.wav", Some(100.0))?;
    engine.tick(0.02);

    thread::sleep(Duration::from_millis(100));

    // Cleanup
    if Path::new("tests/assets/test_voice_duck.wav").exists() {
        fs::remove_file("tests/assets/test_voice_duck.wav")?;
    }

    Ok(())
}

// ============================================================================
// Advanced Edge Case 3: Music Crossfade State Machine Edge Cases
// ============================================================================

#[test]
#[serial]
fn test_music_crossfade_negative_duration() -> Result<()> {
    let mut engine = AudioEngine::new()?;

    test_asset_generator::generate_test_music("tests/assets/test_music_neg.wav", 2.0, 22050)?;

    let track = MusicTrack {
        path: "tests/assets/test_music_neg.wav".to_string(),
        looped: true,
    };

    // Negative crossfade duration (should be treated as 0 or clamped)
    let result = engine.play_music(track, -1.0);
    assert!(
        result.is_ok(),
        "Should handle negative crossfade duration gracefully"
    );

    engine.tick(0.05);
    thread::sleep(Duration::from_millis(50));

    // Cleanup
    fs::remove_file("tests/assets/test_music_neg.wav")?;

    Ok(())
}

#[test]
#[serial]
fn test_music_tick_without_crossfade() -> Result<()> {
    let mut engine = AudioEngine::new()?;

    test_asset_generator::generate_test_music("tests/assets/test_music_simple.wav", 1.0, 22050)?;

    let track = MusicTrack {
        path: "tests/assets/test_music_simple.wav".to_string(),
        looped: false,
    };

    // Play without crossfade (crossfade_sec = 0.0)
    engine.play_music(track, 0.0)?;

    // Tick multiple times to exercise tick logic without crossfade
    for _ in 0..20 {
        engine.tick(0.05);
        thread::sleep(Duration::from_millis(10));
    }

    // Cleanup
    fs::remove_file("tests/assets/test_music_simple.wav")?;

    Ok(())
}

// ============================================================================
// Advanced Edge Case 4: Spatial Audio Volume Calculation Boundaries
// ============================================================================

#[test]
#[serial]
fn test_spatial_volume_zero_master() -> Result<()> {
    let mut engine = AudioEngine::new()?;

    test_asset_generator::generate_test_beep("tests/assets/test_beep_vol.wav", 440.0, 0.3, 22050)?;

    // Set master volume to zero
    engine.set_master_volume(0.0);

    let emitter_id = 2000;
    let pos = Vec3::new(10.0, 0.0, 0.0);

    engine.play_sfx_3d_file(emitter_id, "tests/assets/test_beep_vol.wav", pos)?;

    // Tick with zero master volume
    for _ in 0..10 {
        engine.tick(0.05);
    }

    // Cleanup
    fs::remove_file("tests/assets/test_beep_vol.wav")?;

    Ok(())
}

#[test]
#[serial]
fn test_spatial_volume_maximum_master() -> Result<()> {
    let mut engine = AudioEngine::new()?;

    test_asset_generator::generate_test_beep("tests/assets/test_beep_max.wav", 440.0, 0.3, 22050)?;

    // Set master volume to maximum
    engine.set_master_volume(1.0);

    let emitter_id = 2001;
    let pos = Vec3::ZERO; // At listener position

    engine.play_sfx_3d_file(emitter_id, "tests/assets/test_beep_max.wav", pos)?;

    // Tick with maximum volume
    for _ in 0..10 {
        engine.tick(0.05);
    }

    // Cleanup
    fs::remove_file("tests/assets/test_beep_max.wav")?;

    Ok(())
}

// ============================================================================
// Advanced Edge Case 5: Voice Bank Load Function with Nonexistent TOML
// ============================================================================

#[test]
#[serial]
fn test_voice_bank_load_nonexistent_file() -> Result<()> {
    let result = astraweave_audio::load_voice_bank("nonexistent_voice_bank.toml");

    assert!(
        result.is_err(),
        "Should return error for nonexistent voice bank file"
    );

    Ok(())
}

// ============================================================================
// Advanced Edge Case 6: Dialogue Override with Nonexistent Override File
// ============================================================================

#[test]
#[serial]
fn test_dialogue_override_file_not_found() -> Result<()> {
    fs::create_dir_all("tests/assets/override_test")?;

    // Create DialogueAudioMap pointing to nonexistent file
    let toml_data = r#"
[map.test_dlg]
n0 = "tests/assets/override_test/NONEXISTENT.wav"
"#;

    let toml_path = "tests/assets/override_test/override_map.toml";
    let mut file = File::create(toml_path)?;
    file.write_all(toml_data.as_bytes())?;
    drop(file);

    let audio_map = astraweave_audio::load_dialogue_audio_map(toml_path)?;

    // Create empty VoiceBank
    let bank = astraweave_audio::VoiceBank {
        speakers: std::collections::HashMap::new(),
    };

    let dialogue = Dialogue {
        id: "test_dlg".into(),
        start: "n0".into(),
        nodes: vec![Node {
            id: "n0".into(),
            line: Some(Line {
                speaker: "test_speaker".into(),
                text: "Override should not exist".into(),
                set_vars: vec![],
            }),
            choices: vec![],
            end: true,
        }],
    };
    let state = DialogueState::new(&dialogue);

    let mut audio_engine = AudioEngine::new()?;

    {
        let mut player = DialoguePlayer {
            audio: &mut audio_engine,
            bank: &bank,
            overrides: Some(&audio_map),
            tts: None,
            subtitle_out: None,
        };

        // Should fall back to beep when override file doesn't exist
        let result = player.speak_current(&dialogue, &state)?;
        assert!(
            result,
            "Should fall back to beep when override file missing"
        );
    }

    audio_engine.tick(0.05);
    thread::sleep(Duration::from_millis(50));

    // Cleanup
    let _ = fs::remove_dir_all("tests/assets/override_test");

    Ok(())
}

// ============================================================================
// Advanced Edge Case 7: Multiple Rapid Volume Changes
// ============================================================================

#[test]
#[serial]
fn test_rapid_master_volume_changes() -> Result<()> {
    let mut engine = AudioEngine::new()?;
    // Private directory. A shared `tests/assets` path races with
    // `cleanup_test_assets`, which deletes that tree from another test.
    let dir = tempfile::tempdir()?;
    let wav = dir.path().join("rapid.wav");
    let wav_str = wav
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("temp wav path is not utf-8"))?;

    test_asset_generator::generate_test_beep(wav_str, 440.0, 1.0, 22050)?;

    engine.play_sfx_file(wav_str)?;

    for i in 0..20 {
        let volume = (i as f32 / 20.0).clamp(0.0, 1.0);
        engine.set_master_volume(volume);
        engine.tick(0.02);
        thread::sleep(Duration::from_millis(10));
    }

    Ok(())
}
