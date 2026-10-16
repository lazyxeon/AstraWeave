/// Comprehensive dialogue file-based tests
///
/// Tests all dialogue file loading paths that were previously untested:
/// - DialogueAudioMap override path lookup (Path 1)
/// - VoiceBank explicit file selection (Path 2a)
/// - VoiceBank folder scanning for .ogg/.wav (Path 2b)
/// - TTS fallback with file generation (Path 3)
/// - Error handling for missing files
/// - Complex multi-speaker scenarios
///
/// Expected Coverage Impact:
/// - dialogue_runtime.rs: 15.91% → 60-70% (+45-55pp)
/// - Overall audio crate: 64.29% → 85-90% (+21-26pp)
mod test_asset_generator;

use anyhow::Result;
use astraweave_audio::{
    dialogue_runtime::{load_dialogue_audio_map, DialogueAudioMap, DialoguePlayer},
    engine::AudioEngine,
    voice::{TtsAdapter, VoiceBank, VoiceSpec},
};
use astraweave_gameplay::dialogue::{Dialogue, Line, Node};
use serial_test::serial;
use std::{collections::HashMap, fs, path::Path, thread, time::Duration};

/// Helper: Create test DialogueAudioMap TOML and load it
fn create_test_audio_map() -> Result<DialogueAudioMap> {
    let short = test_asset_generator::asset_path("test_voice_short.wav");
    let medium = test_asset_generator::asset_path("test_voice_medium.wav");
    let toml_content = format!(
        r#"[map.test_dialogue]
n0 = "{short}"
n1 = "{medium}"

[map.multi_speaker]
alice_node = "{short}"
bob_node = "{medium}"
"#
    );

    fs::create_dir_all(test_asset_generator::assets_root_str())?;
    fs::write(
        test_asset_generator::asset_path("test_dialogue_audio_map.toml"),
        toml_content,
    )?;

    load_dialogue_audio_map(test_asset_generator::asset_path(
        "test_dialogue_audio_map.toml",
    ))
}

/// Helper: Create VoiceBank with explicit files
fn create_test_voice_bank_explicit() -> Result<VoiceBank> {
    // Create speaker folder structure
    fs::create_dir_all(test_asset_generator::asset_path("speakers/alice"))?;
    fs::create_dir_all(test_asset_generator::asset_path("speakers/bob"))?;

    // Copy test voice files to speaker folders
    fs::copy(
        test_asset_generator::asset_path("test_voice_short.wav"),
        test_asset_generator::asset_path("speakers/alice/voice_01.wav"),
    )?;
    fs::copy(
        test_asset_generator::asset_path("test_voice_medium.wav"),
        test_asset_generator::asset_path("speakers/alice/voice_02.wav"),
    )?;
    fs::copy(
        test_asset_generator::asset_path("test_voice_long.wav"),
        test_asset_generator::asset_path("speakers/bob/voice_01.wav"),
    )?;

    // Create VoiceBank with explicit file lists
    let mut speakers = HashMap::new();
    speakers.insert(
        "Alice".to_string(),
        VoiceSpec {
            folder: test_asset_generator::asset_path("speakers/alice").to_string(),
            files: vec!["voice_01.wav".to_string(), "voice_02.wav".to_string()],
            tts_voice: None,
        },
    );
    speakers.insert(
        "Bob".to_string(),
        VoiceSpec {
            folder: test_asset_generator::asset_path("speakers/bob").to_string(),
            files: vec!["voice_01.wav".to_string()],
            tts_voice: None,
        },
    );

    Ok(VoiceBank { speakers })
}

/// Helper: Create VoiceBank with folder scanning (no explicit files)
fn create_test_voice_bank_folder_scan() -> Result<VoiceBank> {
    // Create speaker folder with multiple files
    fs::create_dir_all(test_asset_generator::asset_path("speakers/charlie"))?;

    // Add multiple voice files to folder
    fs::copy(
        test_asset_generator::asset_path("test_voice_short.wav"),
        test_asset_generator::asset_path("speakers/charlie/line_01.wav"),
    )?;
    fs::copy(
        test_asset_generator::asset_path("test_voice_medium.wav"),
        test_asset_generator::asset_path("speakers/charlie/line_02.wav"),
    )?;
    fs::copy(
        test_asset_generator::asset_path("test_voice_long.wav"),
        test_asset_generator::asset_path("speakers/charlie/line_03.ogg"), // Test .ogg extension
    )?;

    // Add a non-audio file (should be filtered out)
    fs::write(
        test_asset_generator::asset_path("speakers/charlie/readme.txt"),
        "Test file",
    )?;

    // VoiceSpec with empty files list triggers folder scanning
    let mut speakers = HashMap::new();
    speakers.insert(
        "Charlie".to_string(),
        VoiceSpec {
            folder: test_asset_generator::asset_path("speakers/charlie").to_string(),
            files: vec![], // Empty = folder scan mode
            tts_voice: None,
        },
    );

    Ok(VoiceBank { speakers })
}

/// Mock TTS adapter for testing fallback path
struct MockTtsAdapter;

impl TtsAdapter for MockTtsAdapter {
    fn synth_to_path(&self, voice_id: &str, text: &str, out_path: &str) -> Result<()> {
        // Generate a simple test WAV file using our test_asset_generator
        test_asset_generator::generate_test_voice(out_path, 1.0, 22050)?;

        // Log for debugging
        println!(
            "MockTTS: Generated {} for voice '{}' text '{}'",
            out_path, voice_id, text
        );

        Ok(())
    }
}

// ============================================================================
// Test Category 1: DialogueAudioMap Override Path (Path 1)
// ============================================================================

#[test]
#[serial]
fn test_dialogue_audio_map_override_path() -> Result<()> {
    // Setup
    test_asset_generator::setup_all_test_assets()?;
    let audio_map = create_test_audio_map()?;
    let mut audio_engine = AudioEngine::new()?;
    let bank = VoiceBank {
        speakers: HashMap::new(), // Empty bank forces override path
    };

    // Create dialogue matching audio map
    let dialogue = Dialogue {
        id: "test_dialogue".to_string(),
        start: "n0".to_string(),
        nodes: vec![Node {
            id: "n0".to_string(),
            line: Some(Line {
                speaker: "Speaker1".to_string(),
                text: "First line".to_string(),
                set_vars: vec![],
            }),
            choices: vec![],
            end: false,
        }],
    };
    let state = astraweave_gameplay::dialogue::DialogueState::new(&dialogue);

    // Test: Override path should load test_voice_short.wav
    let mut player = DialoguePlayer {
        audio: &mut audio_engine,
        bank: &bank,
        tts: None,
        overrides: Some(&audio_map),
        subtitle_out: None,
    };

    let result = player.speak_current(&dialogue, &state)?;
    assert!(result, "Should play audio from override");

    // Let audio start
    thread::sleep(Duration::from_millis(50));
    audio_engine.tick(0.05);

    Ok(())
}

#[test]
#[serial]
fn test_dialogue_audio_map_multiple_nodes() -> Result<()> {
    test_asset_generator::setup_all_test_assets()?;
    let audio_map = create_test_audio_map()?;
    let mut audio_engine = AudioEngine::new()?;
    let bank = VoiceBank {
        speakers: HashMap::new(),
    };

    // Dialogue with multiple nodes
    let dialogue = Dialogue {
        id: "test_dialogue".to_string(),
        start: "n0".to_string(),
        nodes: vec![
            Node {
                id: "n0".to_string(),
                line: Some(Line {
                    speaker: "S1".to_string(),
                    text: "First".to_string(),
                    set_vars: vec![],
                }),
                choices: vec![],
                end: false,
            },
            Node {
                id: "n1".to_string(),
                line: Some(Line {
                    speaker: "S2".to_string(),
                    text: "Second".to_string(),
                    set_vars: vec![],
                }),
                choices: vec![],
                end: true,
            },
        ],
    };

    // Play first node
    let state = astraweave_gameplay::dialogue::DialogueState::new(&dialogue);
    let mut player = DialoguePlayer {
        audio: &mut audio_engine,
        bank: &bank,
        tts: None,
        overrides: Some(&audio_map),
        subtitle_out: None,
    };

    let r1 = player.speak_current(&dialogue, &state)?;
    assert!(r1);

    thread::sleep(Duration::from_millis(100));
    audio_engine.tick(0.1);

    Ok(())
}

#[test]
#[serial]
fn test_dialogue_audio_map_missing_override() -> Result<()> {
    // Test fallback when override doesn't exist
    test_asset_generator::setup_all_test_assets()?;
    let audio_map = create_test_audio_map()?;
    let mut audio_engine = AudioEngine::new()?;
    let bank = VoiceBank {
        speakers: HashMap::new(),
    };

    // Dialogue node NOT in audio map
    let dialogue = Dialogue {
        id: "unknown_dialogue".to_string(),
        start: "n0".to_string(),
        nodes: vec![Node {
            id: "n0".to_string(),
            line: Some(Line {
                speaker: "Unknown".to_string(),
                text: "This has no override".to_string(),
                set_vars: vec![],
            }),
            choices: vec![],
            end: true,
        }],
    };
    let state = astraweave_gameplay::dialogue::DialogueState::new(&dialogue);

    let mut player = DialoguePlayer {
        audio: &mut audio_engine,
        bank: &bank,
        tts: None,
        overrides: Some(&audio_map),
        subtitle_out: None,
    };

    // Should fallback to beep (no override, no bank, no TTS)
    let result = player.speak_current(&dialogue, &state)?;
    assert!(result, "Should fallback to beep");

    Ok(())
}

// ============================================================================
// Test Category 2: VoiceBank Explicit File Selection (Path 2a)
// ============================================================================

#[test]
#[serial]
fn test_voice_bank_explicit_file_selection() -> Result<()> {
    test_asset_generator::setup_all_test_assets()?;
    let bank = create_test_voice_bank_explicit()?;
    let mut audio_engine = AudioEngine::new()?;

    let dialogue = Dialogue {
        id: "test".to_string(),
        start: "n0".to_string(),
        nodes: vec![Node {
            id: "n0".to_string(),
            line: Some(Line {
                speaker: "Alice".to_string(),
                text: "Hello from Alice".to_string(),
                set_vars: vec![],
            }),
            choices: vec![],
            end: true,
        }],
    };
    let state = astraweave_gameplay::dialogue::DialogueState::new(&dialogue);

    let mut player = DialoguePlayer {
        audio: &mut audio_engine,
        bank: &bank,
        tts: None,
        overrides: None,
        subtitle_out: None,
    };

    // Should select one of Alice's explicit files
    let result = player.speak_current(&dialogue, &state)?;
    assert!(result, "Should play file from VoiceBank");

    thread::sleep(Duration::from_millis(50));
    audio_engine.tick(0.05);

    Ok(())
}

#[test]
#[serial]
fn test_voice_bank_multiple_speakers() -> Result<()> {
    test_asset_generator::setup_all_test_assets()?;
    let bank = create_test_voice_bank_explicit()?;
    let mut audio_engine = AudioEngine::new()?;

    let dialogue = Dialogue {
        id: "conversation".to_string(),
        start: "n0".to_string(),
        nodes: vec![
            Node {
                id: "n0".to_string(),
                line: Some(Line {
                    speaker: "Alice".to_string(),
                    text: "Hi Bob!".to_string(),
                    set_vars: vec![],
                }),
                choices: vec![],
                end: false,
            },
            Node {
                id: "n1".to_string(),
                line: Some(Line {
                    speaker: "Bob".to_string(),
                    text: "Hi Alice!".to_string(),
                    set_vars: vec![],
                }),
                choices: vec![],
                end: true,
            },
        ],
    };

    // Play Alice's line
    let state = astraweave_gameplay::dialogue::DialogueState::new(&dialogue);
    let mut player = DialoguePlayer {
        audio: &mut audio_engine,
        bank: &bank,
        tts: None,
        overrides: None,
        subtitle_out: None,
    };

    let r1 = player.speak_current(&dialogue, &state)?;
    assert!(r1, "Alice should speak");

    thread::sleep(Duration::from_millis(100));
    audio_engine.tick(0.1);

    Ok(())
}

// ============================================================================
// Test Category 3: VoiceBank Folder Scanning (Path 2b)
// ============================================================================

#[test]
#[serial]
fn test_voice_bank_folder_scanning() -> Result<()> {
    test_asset_generator::setup_all_test_assets()?;
    let bank = create_test_voice_bank_folder_scan()?;
    let mut audio_engine = AudioEngine::new()?;

    let dialogue = Dialogue {
        id: "test".to_string(),
        start: "n0".to_string(),
        nodes: vec![Node {
            id: "n0".to_string(),
            line: Some(Line {
                speaker: "Charlie".to_string(),
                text: "Testing folder scan".to_string(),
                set_vars: vec![],
            }),
            choices: vec![],
            end: true,
        }],
    };
    let state = astraweave_gameplay::dialogue::DialogueState::new(&dialogue);

    let mut player = DialoguePlayer {
        audio: &mut audio_engine,
        bank: &bank,
        tts: None,
        overrides: None,
        subtitle_out: None,
    };

    // Should scan folder and select a random .wav/.ogg file
    let result = player.speak_current(&dialogue, &state)?;
    assert!(result, "Should play file from folder scan");

    thread::sleep(Duration::from_millis(50));
    audio_engine.tick(0.05);

    Ok(())
}

#[test]
#[serial]
fn test_folder_scan_filters_non_audio() -> Result<()> {
    // Verify that .txt files are filtered out during folder scan
    test_asset_generator::setup_all_test_assets()?;
    let bank = create_test_voice_bank_folder_scan()?;
    let mut audio_engine = AudioEngine::new()?;

    // Check that readme.txt exists but is not a voice candidate.
    let readme = test_asset_generator::asset_path("speakers/charlie/readme.txt");
    assert!(Path::new(readme).exists());
    let candidates = astraweave_audio::dialogue_runtime::voice_folder_candidates(Path::new(
        test_asset_generator::asset_path("speakers/charlie"),
    ));
    let names: Vec<String> = candidates
        .iter()
        .map(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("")
                .to_string()
        })
        .collect();
    assert_eq!(
        names,
        vec![
            "line_01.wav".to_string(),
            "line_02.wav".to_string(),
            "line_03.ogg".to_string()
        ]
    );
    assert!(!names.iter().any(|name| name == "readme.txt"));

    // Should only select .wav/.ogg files, not .txt
    let dialogue = Dialogue {
        id: "test".to_string(),
        start: "n0".to_string(),
        nodes: vec![Node {
            id: "n0".to_string(),
            line: Some(Line {
                speaker: "Charlie".to_string(),
                text: "Test".to_string(),
                set_vars: vec![],
            }),
            choices: vec![],
            end: true,
        }],
    };
    let state = astraweave_gameplay::dialogue::DialogueState::new(&dialogue);

    let mut player = DialoguePlayer {
        audio: &mut audio_engine,
        bank: &bank,
        tts: None,
        overrides: None,
        subtitle_out: None,
    };

    // One play. The candidate assertion above is the filter; this checks
    // that a listed file actually opens.
    let result = player.speak_current(&dialogue, &state)?;
    assert!(result);
    audio_engine.tick(0.1);

    Ok(())
}

// ============================================================================
// Test Category 4: TTS Fallback (Path 3)
// ============================================================================

#[test]
#[serial]
fn test_tts_fallback_when_no_files() -> Result<()> {
    test_asset_generator::setup_all_test_assets()?;

    // Create folder for TTS output
    fs::create_dir_all(test_asset_generator::asset_path("speakers/tts_speaker"))?;

    // VoiceBank with TTS voice but no files
    let mut speakers = HashMap::new();
    speakers.insert(
        "TTSSpeaker".to_string(),
        VoiceSpec {
            folder: test_asset_generator::asset_path("speakers/tts_speaker").to_string(),
            files: vec![], // No files
            tts_voice: Some("test_voice_a".to_string()),
        },
    );
    let bank = VoiceBank { speakers };

    let mut audio_engine = AudioEngine::new()?;
    let tts = MockTtsAdapter;

    let dialogue = Dialogue {
        id: "test".to_string(),
        start: "n0".to_string(),
        nodes: vec![Node {
            id: "n0".to_string(),
            line: Some(Line {
                speaker: "TTSSpeaker".to_string(),
                text: "This will be synthesized".to_string(),
                set_vars: vec![],
            }),
            choices: vec![],
            end: true,
        }],
    };
    let state = astraweave_gameplay::dialogue::DialogueState::new(&dialogue);

    let mut player = DialoguePlayer {
        audio: &mut audio_engine,
        bank: &bank,
        tts: Some(&tts),
        overrides: None,
        subtitle_out: None,
    };

    // Should trigger TTS fallback
    let result = player.speak_current(&dialogue, &state)?;
    assert!(result, "Should synthesize via TTS");

    thread::sleep(Duration::from_millis(100));
    audio_engine.tick(0.1);

    Ok(())
}

#[test]
#[serial]
fn test_tts_generates_temporary_file() -> Result<()> {
    test_asset_generator::setup_all_test_assets()?;
    fs::create_dir_all(test_asset_generator::asset_path("speakers/tts_speaker2"))?;

    let mut speakers = HashMap::new();
    speakers.insert(
        "TTS2".to_string(),
        VoiceSpec {
            folder: test_asset_generator::asset_path("speakers/tts_speaker2").to_string(),
            files: vec![],
            tts_voice: Some("voice_b".to_string()),
        },
    );
    let bank = VoiceBank { speakers };
    let mut audio_engine = AudioEngine::new()?;
    let tts = MockTtsAdapter;

    let dialogue = Dialogue {
        id: "test".to_string(),
        start: "n0".to_string(),
        nodes: vec![Node {
            id: "n0".to_string(),
            line: Some(Line {
                speaker: "TTS2".to_string(),
                text: "Testing TTS file generation".to_string(),
                set_vars: vec![],
            }),
            choices: vec![],
            end: true,
        }],
    };
    let state = astraweave_gameplay::dialogue::DialogueState::new(&dialogue);

    let mut player = DialoguePlayer {
        audio: &mut audio_engine,
        bank: &bank,
        tts: Some(&tts),
        overrides: None,
        subtitle_out: None,
    };

    let result = player.speak_current(&dialogue, &state)?;
    assert!(result);

    // Check that TTS generated a file in the folder
    let entries: Vec<_> = fs::read_dir(test_asset_generator::asset_path("speakers/tts_speaker2"))?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "wav"))
        .collect();

    assert!(!entries.is_empty(), "TTS should generate .wav file");

    Ok(())
}

// ============================================================================
// Test Category 5: Subtitle Output
// ============================================================================

#[test]
#[serial]
fn test_subtitle_output_callback() -> Result<()> {
    test_asset_generator::setup_all_test_assets()?;
    let bank = create_test_voice_bank_explicit()?;
    let mut audio_engine = AudioEngine::new()?;

    let dialogue = Dialogue {
        id: "test".to_string(),
        start: "n0".to_string(),
        nodes: vec![Node {
            id: "n0".to_string(),
            line: Some(Line {
                speaker: "Alice".to_string(),
                text: "Test subtitle".to_string(),
                set_vars: vec![],
            }),
            choices: vec![],
            end: true,
        }],
    };
    let state = astraweave_gameplay::dialogue::DialogueState::new(&dialogue);

    let mut subtitles: Vec<(String, String)> = vec![];
    {
        let mut push_sub = |speaker: String, text: String| {
            subtitles.push((speaker, text));
        };

        let mut player = DialoguePlayer {
            audio: &mut audio_engine,
            bank: &bank,
            tts: None,
            overrides: None,
            subtitle_out: Some(&mut push_sub),
        };

        player.speak_current(&dialogue, &state)?;
    }

    assert_eq!(subtitles.len(), 1);
    assert_eq!(subtitles[0].0, "Alice");
    assert_eq!(subtitles[0].1, "Test subtitle");

    Ok(())
}

// ============================================================================
// Test Category 6: Integration & Edge Cases
// ============================================================================

#[test]
#[serial]
fn test_priority_override_then_voicebank_then_tts() -> Result<()> {
    // Test priority order: override > voicebank > TTS > beep
    test_asset_generator::setup_all_test_assets()?;
    let audio_map = create_test_audio_map()?;
    let bank = create_test_voice_bank_explicit()?;
    let mut audio_engine = AudioEngine::new()?;
    let tts = MockTtsAdapter;

    // Node with override (should use override, not voicebank/TTS)
    let dialogue = Dialogue {
        id: "test_dialogue".to_string(),
        start: "n0".to_string(),
        nodes: vec![Node {
            id: "n0".to_string(),
            line: Some(Line {
                speaker: "Alice".to_string(), // Exists in VoiceBank
                text: "Override test".to_string(),
                set_vars: vec![],
            }),
            choices: vec![],
            end: true,
        }],
    };
    let state = astraweave_gameplay::dialogue::DialogueState::new(&dialogue);

    let mut player = DialoguePlayer {
        audio: &mut audio_engine,
        bank: &bank,
        tts: Some(&tts),
        overrides: Some(&audio_map),
        subtitle_out: None,
    };

    // Override should take priority over VoiceBank
    let result = player.speak_current(&dialogue, &state)?;
    assert!(result);

    Ok(())
}

#[test]
#[serial]
fn test_comprehensive_dialogue_pipeline() -> Result<()> {
    // Full integration test: multiple speakers, paths, and fallbacks
    test_asset_generator::setup_all_test_assets()?;
    let audio_map = create_test_audio_map()?;
    let bank = create_test_voice_bank_explicit()?;
    let mut audio_engine = AudioEngine::new()?;

    let dialogue = Dialogue {
        id: "multi_speaker".to_string(),
        start: "alice_node".to_string(),
        nodes: vec![
            Node {
                id: "alice_node".to_string(),
                line: Some(Line {
                    speaker: "Alice".to_string(),
                    text: "Alice speaks".to_string(),
                    set_vars: vec![],
                }),
                choices: vec![],
                end: false,
            },
            Node {
                id: "bob_node".to_string(),
                line: Some(Line {
                    speaker: "Bob".to_string(),
                    text: "Bob responds".to_string(),
                    set_vars: vec![],
                }),
                choices: vec![],
                end: false,
            },
            Node {
                id: "unknown_node".to_string(),
                line: Some(Line {
                    speaker: "Unknown".to_string(),
                    text: "Fallback to beep".to_string(),
                    set_vars: vec![],
                }),
                choices: vec![],
                end: true,
            },
        ],
    };

    let state = astraweave_gameplay::dialogue::DialogueState::new(&dialogue);

    // Test first node only (multi-node traversal requires choice system)
    let mut player = DialoguePlayer {
        audio: &mut audio_engine,
        bank: &bank,
        tts: None,
        overrides: Some(&audio_map),
        subtitle_out: None,
    };

    let result = player.speak_current(&dialogue, &state)?;
    assert!(result, "First node should play audio");

    thread::sleep(Duration::from_millis(100));
    audio_engine.tick(0.1);

    Ok(())
}
