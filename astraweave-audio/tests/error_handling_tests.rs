//! Error handling and edge case tests for astraweave-audio
//!
//! This test suite validates error recovery, boundary conditions, and defensive
//! programming paths that aren't exercised by normal-path tests.
//!
//! Coverage Targets:
//! - Malformed TOML parsing (dialogue_audio_map, voice_bank)
//! - Missing/corrupted audio files
//! - File permission errors
//! - Empty folders (folder scan edge case)
//! - TTS adapter failures
//! - Extreme spatial audio positions (NaN, Inf, very large values)
//! - Rapid crossfade edge cases

use anyhow::Result;
use astraweave_audio::{AudioEngine, DialoguePlayer, MusicTrack};
use astraweave_gameplay::{Dialogue, DialogueState};
use glam::Vec3;
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use std::thread;
use std::time::Duration;

#[path = "test_asset_generator.rs"]
mod test_asset_generator;
use test_asset_generator::generate_test_beep;

// ============================================================================
// Setup & Cleanup Helpers
// ============================================================================

/// Generate unique test directory path to prevent parallel test race conditions
fn unique_test_dir(test_name: &str) -> String {
    format!("target/test_assets/audio_error_tests/{}", test_name)
}

fn setup_error_test_assets_named(test_name: &str) -> Result<String> {
    let base_dir = unique_test_dir(test_name);
    fs::create_dir_all(&base_dir)?;
    fs::create_dir_all(format!("{}/malformed", base_dir))?;
    fs::create_dir_all(format!("{}/empty_folder", base_dir))?;
    fs::create_dir_all(format!("{}/corrupted", base_dir))?;
    Ok(base_dir)
}

fn cleanup_error_test_assets_named(base_dir: &str) -> Result<()> {
    if Path::new(base_dir).exists() {
        // Ignore permission errors during cleanup (Windows file locking)
        let _ = fs::remove_dir_all(base_dir);
    }
    Ok(())
}

fn setup_error_test_assets() -> Result<()> {
    fs::create_dir_all(test_asset_generator::asset_path("error_tests"))?;
    fs::create_dir_all(test_asset_generator::asset_path("error_tests/malformed"))?;
    fs::create_dir_all(test_asset_generator::asset_path("error_tests/empty_folder"))?;
    fs::create_dir_all(test_asset_generator::asset_path("error_tests/corrupted"))?;
    Ok(())
}

fn cleanup_error_test_assets() -> Result<()> {
    if Path::new(test_asset_generator::asset_path("error_tests")).exists() {
        // Ignore permission errors during cleanup (Windows file locking)
        let _ = fs::remove_dir_all(test_asset_generator::asset_path("error_tests"));
    }
    Ok(())
}

// ============================================================================
// Error Category 1: TOML Parsing Errors
// ============================================================================

#[test]
fn test_malformed_toml_dialogue_audio_map() -> Result<()> {
    setup_error_test_assets()?;

    // Create malformed TOML (missing closing bracket, invalid syntax)
    let malformed_toml = r#"
[map]
test_dialogue = { n0 = "test.wav"  # Missing closing brace
"#;

    let toml_path = test_asset_generator::asset_path("error_tests/malformed/bad_dialogue_map.toml");
    let mut file = File::create(toml_path)?;
    file.write_all(malformed_toml.as_bytes())?;
    drop(file);

    // Attempt to load malformed TOML
    let result = astraweave_audio::load_dialogue_audio_map(toml_path);

    // Should return error (either parsing or validation)
    assert!(
        result.is_err(),
        "Should fail to load malformed TOML, got: {:?}",
        result
    );

    cleanup_error_test_assets()?;
    Ok(())
}

#[test]
fn test_malformed_toml_voice_bank() -> Result<()> {
    let test_dir = setup_error_test_assets_named("malformed_voice_bank")?;

    // Create malformed VoiceBank TOML (invalid key-value structure)
    let malformed_toml = r#"
[speakers
alice = { folder = "speakers/alice", files = ["voice.wav"] }
"#; // Missing closing bracket for [speakers]

    let toml_path = format!("{}/malformed/bad_voice_bank.toml", test_dir);
    let mut file = File::create(&toml_path)?;
    file.write_all(malformed_toml.as_bytes())?;
    drop(file);

    // Attempt to load malformed TOML
    let result = astraweave_audio::load_voice_bank(&toml_path);

    // Should return error
    assert!(
        result.is_err(),
        "Should fail to load malformed VoiceBank TOML, got: {:?}",
        result
    );

    cleanup_error_test_assets_named(&test_dir)?;
    Ok(())
}

// ============================================================================
// Error Category 2: Missing/Corrupted Files
// ============================================================================

#[test]
fn test_missing_audio_file_play_sfx() -> Result<()> {
    let mut engine = AudioEngine::new()?;

    // Attempt to play non-existent file
    let result = engine.play_sfx_file(test_asset_generator::asset_path("NONEXISTENT_FILE.wav"));

    // Should return error (file not found)
    assert!(
        result.is_err(),
        "Should fail for non-existent file, got: {:?}",
        result
    );

    Ok(())
}

#[test]
fn test_corrupted_audio_file() -> Result<()> {
    setup_error_test_assets()?;

    // Create "corrupted" WAV file (invalid header, truncated data)
    let corrupted_path = test_asset_generator::asset_path("error_tests/corrupted/bad.wav");
    let mut file = File::create(corrupted_path)?;
    file.write_all(b"NOT A REAL WAV FILE HEADER\x00\x00")?;
    drop(file);

    let mut engine = AudioEngine::new()?;

    // Attempt to play corrupted file
    let result = engine.play_sfx_file(corrupted_path);

    // Rodio should reject invalid WAV format
    assert!(
        result.is_err(),
        "Should fail for corrupted audio file, got: {:?}",
        result
    );

    cleanup_error_test_assets()?;
    Ok(())
}

#[test]
fn test_empty_folder_voice_bank_scan() -> Result<()> {
    setup_error_test_assets()?;

    // Create VoiceBank with empty folder (no .wav/.ogg files)
    let empty_folder = test_asset_generator::asset_path("error_tests/empty_folder");
    let voice_bank_toml = format!(
        r#"[speakers.empty_speaker]
folder = "{empty_folder}"
files = []  # Triggers folder scan
"#
    );

    let toml_path = test_asset_generator::asset_path("error_tests/empty_voice_bank.toml");
    let mut file = File::create(toml_path)?;
    file.write_all(voice_bank_toml.as_bytes())?;
    drop(file);

    let bank = astraweave_audio::load_voice_bank(toml_path)?;

    // Create simple dialogue using correct structure
    use astraweave_gameplay::dialogue::{Line, Node};

    let dialogue = Dialogue {
        id: "empty_test".into(),
        start: "n0".into(),
        nodes: vec![Node {
            id: "n0".into(),
            line: Some(Line {
                speaker: "empty_speaker".into(),
                text: "Hello".into(),
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

        // Attempt to speak with empty folder (should fall back to beep)
        let result = player.speak_current(&dialogue, &state)?;
        assert!(result, "Should fall back to beep when folder is empty");
    }

    audio_engine.tick(0.05);
    thread::sleep(Duration::from_millis(50));

    cleanup_error_test_assets()?;
    Ok(())
}

// ============================================================================
// Error Category 3: TTS Adapter Failures
// ============================================================================

/// Mock TTS adapter that always fails (simulates network error, API failure)
struct FailingTtsAdapter;

impl astraweave_audio::TtsAdapter for FailingTtsAdapter {
    fn synth_to_path(&self, _voice_id: &str, _text: &str, _out_path: &str) -> Result<()> {
        anyhow::bail!("Simulated TTS failure: API timeout");
    }
}

#[test]
fn test_tts_adapter_failure_fallback() -> Result<()> {
    setup_error_test_assets()?;

    // Create VoiceBank with TTS but no files (forces TTS path)
    let tts_fail = test_asset_generator::asset_path("error_tests/tts_fail");
    let voice_bank_toml = format!(
        r#"[speakers.tts_fail_speaker]
folder = "{tts_fail}"
files = []
tts_voice = "en-US-Neural"
"#
    );

    fs::create_dir_all(test_asset_generator::asset_path("error_tests/tts_fail"))?;

    let toml_path = test_asset_generator::asset_path("error_tests/tts_fail_voice_bank.toml");
    let mut file = File::create(toml_path)?;
    file.write_all(voice_bank_toml.as_bytes())?;
    drop(file);

    let bank = astraweave_audio::load_voice_bank(toml_path)?;

    // Create dialogue using correct structure
    use astraweave_gameplay::dialogue::{Line, Node};

    let dialogue = Dialogue {
        id: "tts_fail_test".into(),
        start: "n0".into(),
        nodes: vec![Node {
            id: "n0".into(),
            line: Some(Line {
                speaker: "tts_fail_speaker".into(),
                text: "This should fail TTS".into(),
                set_vars: vec![],
            }),
            choices: vec![],
            end: true,
        }],
    };
    let state = DialogueState::new(&dialogue);

    let mut audio_engine = AudioEngine::new()?;
    let failing_tts = FailingTtsAdapter;

    {
        let mut player = DialoguePlayer {
            audio: &mut audio_engine,
            bank: &bank,
            overrides: None,
            tts: Some(&failing_tts),
            subtitle_out: None,
        };

        // Attempt to speak - TTS will fail, should fall back to beep
        let result = player.speak_current(&dialogue, &state);

        // DialoguePlayer might propagate error OR fall back to beep
        // Check if it returned Ok (beep fallback) or Err (propagated TTS error)
        match result {
            Ok(spoke) => {
                // Fell back to beep successfully
                assert!(spoke, "Should fall back to beep after TTS failure");
            }
            Err(e) => {
                // TTS error propagated (also valid behavior)
                assert!(
                    e.to_string().contains("TTS") || e.to_string().contains("timeout"),
                    "Error should mention TTS failure: {}",
                    e
                );
            }
        }
    }

    audio_engine.tick(0.05);

    cleanup_error_test_assets()?;
    Ok(())
}

// ============================================================================
// Error Category 4: Extreme Spatial Audio Edge Cases
// ============================================================================

#[test]
fn test_spatial_audio_nan_coordinates() -> Result<()> {
    let mut engine = AudioEngine::new()?;

    // Generate test beep
    generate_test_beep(
        test_asset_generator::asset_path("test_beep_nan.wav"),
        440.0,
        0.2,
        22050,
    )?;

    // Create emitter at NaN position (undefined behavior in 3D math)
    let nan_pos = Vec3::new(f32::NAN, f32::NAN, f32::NAN);
    let emitter_id = 999; // Unique emitter ID

    let result = engine.play_sfx_3d_file(
        emitter_id,
        test_asset_generator::asset_path("test_beep_nan.wav"),
        nan_pos,
    );

    // Engine should either:
    // 1. Reject NaN coordinates (return error)
    // 2. Sanitize to zero/default position
    // 3. Accept but produce silent/undefined audio
    match result {
        Ok(()) => {
            // NaN coordinates accepted - ensure doesn't crash
            engine.tick(0.05);
            println!("NaN position accepted, emitter ID: {}", emitter_id);
        }
        Err(e) => {
            // NaN coordinates rejected (defensive programming)
            println!("NaN position rejected: {}", e);
        }
    }

    // Cleanup
    if Path::new(test_asset_generator::asset_path("test_beep_nan.wav")).exists() {
        fs::remove_file(test_asset_generator::asset_path("test_beep_nan.wav"))?;
    }

    Ok(())
}

#[test]
fn test_spatial_audio_infinite_coordinates() -> Result<()> {
    let mut engine = AudioEngine::new()?;

    // Generate test beep
    generate_test_beep(
        test_asset_generator::asset_path("test_beep_inf.wav"),
        440.0,
        0.2,
        22050,
    )?;

    // Create emitter at infinite distance
    let inf_pos = Vec3::new(f32::INFINITY, f32::INFINITY, f32::INFINITY);
    let emitter_id = 1000;

    let result = engine.play_sfx_3d_file(
        emitter_id,
        test_asset_generator::asset_path("test_beep_inf.wav"),
        inf_pos,
    );

    // Similar to NaN - engine should handle gracefully
    match result {
        Ok(()) => {
            engine.tick(0.05);
            println!("Infinite position accepted, emitter ID: {}", emitter_id);
        }
        Err(e) => {
            println!("Infinite position rejected: {}", e);
        }
    }

    // Cleanup
    if Path::new(test_asset_generator::asset_path("test_beep_inf.wav")).exists() {
        fs::remove_file(test_asset_generator::asset_path("test_beep_inf.wav"))?;
    }

    Ok(())
}

#[test]
fn test_spatial_audio_extreme_distance() -> Result<()> {
    let mut engine = AudioEngine::new()?;

    // Generate test beep
    generate_test_beep(
        test_asset_generator::asset_path("test_beep_extreme.wav"),
        440.0,
        0.5,
        22050,
    )?;

    // Emitter at extreme distance (1 million units away)
    let extreme_pos = Vec3::new(1_000_000.0, 1_000_000.0, 1_000_000.0);
    let emitter_id = 1001;

    let result = engine.play_sfx_3d_file(
        emitter_id,
        test_asset_generator::asset_path("test_beep_extreme.wav"),
        extreme_pos,
    );

    assert!(result.is_ok(), "Should accept extreme but finite position");

    // Update positions and tick
    for _ in 0..30 {
        engine.tick(1.0 / 60.0);
        thread::sleep(Duration::from_millis(16));
    }

    // Sound should be inaudible due to distance attenuation (but not crash)
    println!("Extreme distance emitter created: {}", emitter_id);

    // Cleanup
    if Path::new(test_asset_generator::asset_path("test_beep_extreme.wav")).exists() {
        fs::remove_file(test_asset_generator::asset_path("test_beep_extreme.wav"))?;
    }

    Ok(())
}

// ============================================================================
// Error Category 5: Rapid Crossfade Edge Cases
// ============================================================================

#[test]
fn test_music_instant_crossfade_zero_duration() -> Result<()> {
    let test_dir = setup_error_test_assets_named("instant_crossfade")?;

    let mut engine = AudioEngine::new()?;

    // Generate two test music tracks with unique paths
    let track1_path = format!("{}/test_music_instant_1.wav", test_dir);
    let track2_path = format!("{}/test_music_instant_2.wav", test_dir);
    test_asset_generator::generate_test_music(&track1_path, 2.0, 22050)?;
    test_asset_generator::generate_test_music(&track2_path, 2.0, 22050)?;

    // Play first track
    let track1 = MusicTrack {
        path: track1_path.clone(),
        looped: true,
    };
    engine.play_music(track1, 0.0)?;
    thread::sleep(Duration::from_millis(100));

    // Instant crossfade (0.0 seconds) - edge case
    let track2 = MusicTrack {
        path: track2_path.clone(),
        looped: true,
    };
    let result = engine.play_music(track2, 0.0); // Zero crossfade duration

    assert!(result.is_ok(), "Should handle zero-duration crossfade");

    engine.tick(0.05);
    thread::sleep(Duration::from_millis(100));

    // Stop audio before cleanup to release file handles
    drop(engine);
    thread::sleep(Duration::from_millis(50));

    cleanup_error_test_assets_named(&test_dir)?;
    Ok(())
}

#[test]
fn test_music_very_long_crossfade() -> Result<()> {
    let test_dir = setup_error_test_assets_named("long_crossfade")?;

    let mut engine = AudioEngine::new()?;

    // Generate two test music tracks with unique paths
    let track1_path = format!("{}/test_music_long_1.wav", test_dir);
    let track2_path = format!("{}/test_music_long_2.wav", test_dir);
    test_asset_generator::generate_test_music(&track1_path, 5.0, 22050)?;
    test_asset_generator::generate_test_music(&track2_path, 5.0, 22050)?;

    // Play first track
    let track1 = MusicTrack {
        path: track1_path.clone(),
        looped: true,
    };
    engine.play_music(track1, 0.0)?;
    thread::sleep(Duration::from_millis(100));

    // Very long crossfade (10 seconds, longer than tracks)
    let track2 = MusicTrack {
        path: track2_path.clone(),
        looped: true,
    };
    let result = engine.play_music(track2, 10.0); // 10-second crossfade

    assert!(result.is_ok(), "Should handle very long crossfade duration");

    // Tick a few times to process crossfade state
    for _ in 0..10 {
        engine.tick(0.1);
    }

    // Stop audio before cleanup to release file handles
    drop(engine);
    thread::sleep(Duration::from_millis(50));

    cleanup_error_test_assets_named(&test_dir)?;
    Ok(())
}
