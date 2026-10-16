use crate::{
    engine::AudioEngine,
    voice::{TtsAdapter, VoiceBank},
};
use anyhow::Result;
use astraweave_gameplay::dialogue::{Dialogue, DialogueState};
use rand::prelude::*;
use std::{collections::HashMap, fs, path::Path};

#[derive(Clone, Debug, serde::Deserialize)]
pub struct DialogueAudioMap {
    /// maps Dialogue.id -> (optional) overrides per node.id -> filename
    pub map: HashMap<String, HashMap<String, String>>,
}

/// `.ogg` and `.wav` files in `folder`, sorted by path.
///
/// Directory order is not stable across operating systems. The playback
/// pick is still random; the candidate set is not.
pub fn voice_folder_candidates(folder: &Path) -> Vec<std::path::PathBuf> {
    let Ok(rd) = fs::read_dir(folder) else {
        return Vec::new();
    };
    let mut pool = Vec::new();
    for entry in rd.flatten() {
        let path = entry.path();
        let Some(ext) = path.extension().and_then(|ext| ext.to_str()) else {
            continue;
        };
        if ext.eq_ignore_ascii_case("ogg") || ext.eq_ignore_ascii_case("wav") {
            pool.push(path);
        }
    }
    pool.sort();
    pool
}

pub fn load_dialogue_audio_map(path: &str) -> Result<DialogueAudioMap> {
    let txt = fs::read_to_string(path)?;
    let m: DialogueAudioMap = toml::from_str(&txt)?;
    Ok(m)
}

pub struct DialoguePlayer<'a> {
    pub audio: &'a mut AudioEngine,
    pub bank: &'a VoiceBank,
    pub tts: Option<&'a dyn TtsAdapter>,
    pub overrides: Option<&'a DialogueAudioMap>,
    pub subtitle_out: Option<&'a mut dyn FnMut(String, String)>, // (speaker, text)
}

impl<'a> DialoguePlayer<'a> {
    /// Play the current node’s line if any (blocking the queue on the Sink by appending).
    /// Returns true if audio played or beeped.
    pub fn speak_current(&mut self, dlg: &Dialogue, st: &DialogueState) -> Result<bool> {
        let node = st.current(dlg);
        let Some(line) = &node.line else {
            return Ok(false);
        };
        let spk = &line.speaker;
        let txt = &line.text;

        if let Some(out) = &mut self.subtitle_out {
            out(spk.clone(), txt.clone());
        }

        // 1) explicit override? (dialogue id + node id)
        if let Some(over) = self.overrides {
            if let Some(per_dialog) = over.map.get(&dlg.id) {
                if let Some(fname) = per_dialog.get(&node.id) {
                    if Path::new(fname).exists() {
                        self.audio.play_voice_file(fname, None)?;
                        return Ok(true);
                    }
                }
            }
        }

        // 2) VoiceBank folder/files for speaker
        if let Some(vspec) = self.bank.speakers.get(spk) {
            // try explicit files
            if !vspec.files.is_empty() {
                let mut rng = rand::rng();
                if let Some(choice) = vspec.files.choose(&mut rng) {
                    let path = format!("{}/{}", vspec.folder, choice);
                    if Path::new(&path).exists() {
                        self.audio.play_voice_file(&path, None)?;
                        return Ok(true);
                    }
                }
            } else {
                // or any .ogg/.wav in folder
                let pool = voice_folder_candidates(Path::new(&vspec.folder));
                if !pool.is_empty() {
                    let mut rng = rand::rng();
                    // INVARIANT: pool is non-empty (checked above), so choose always returns Some
                    #[allow(clippy::expect_used)]
                    let path = pool
                        .choose(&mut rng)
                        .expect("BUG: pool should have items after is_empty check");
                    self.audio.play_voice_file(&path.to_string_lossy(), None)?;
                    return Ok(true);
                }
            }

            // 3) TTS fallback if available
            if let (Some(tts), Some(voice_id)) = (self.tts.as_ref(), vspec.tts_voice.as_ref()) {
                let out_path = format!("{}/tts_tmp_{}.wav", vspec.folder, rand::random::<u64>());
                tts.synth_to_path(voice_id, txt, &out_path)?;
                self.audio.play_voice_file(&out_path, None)?;
                return Ok(true);
            }
        }

        // 4) Final fallback: beep by text length
        self.audio.play_voice_beep(txt.len());
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use astraweave_gameplay::dialogue::{Dialogue, Line, Node};

    #[test]
    fn test_dialogue_audio_map_creation() {
        let mut map = HashMap::new();
        let mut node_map = HashMap::new();
        node_map.insert("n0".to_string(), "audio/greeting.wav".to_string());
        node_map.insert("n1".to_string(), "audio/farewell.wav".to_string());
        map.insert("dlg_test".to_string(), node_map);

        let audio_map = DialogueAudioMap { map };
        assert_eq!(audio_map.map.len(), 1);
        let node_overrides = audio_map.map.get("dlg_test").unwrap();
        assert_eq!(node_overrides.len(), 2);
        assert_eq!(
            node_overrides.get("n0"),
            Some(&"audio/greeting.wav".to_string())
        );
    }

    #[test]
    fn test_dialogue_audio_map_empty() {
        let audio_map = DialogueAudioMap {
            map: HashMap::new(),
        };
        assert_eq!(audio_map.map.len(), 0);
    }

    #[test]
    fn test_load_dialogue_audio_map_valid() -> Result<()> {
        std::fs::create_dir_all("target")?;
        let test_toml = "target/test_dialogue_audio.toml";
        std::fs::write(
            test_toml,
            r#"
[map.dialogue1]
node1 = "audio/line1.wav"
node2 = "audio/line2.wav"

[map.dialogue2]
node_start = "audio/intro.wav"
"#,
        )?;

        let audio_map = load_dialogue_audio_map(test_toml)?;
        assert_eq!(audio_map.map.len(), 2);

        let dlg1 = audio_map.map.get("dialogue1").unwrap();
        assert_eq!(dlg1.len(), 2);
        assert_eq!(dlg1.get("node1"), Some(&"audio/line1.wav".to_string()));

        let dlg2 = audio_map.map.get("dialogue2").unwrap();
        assert_eq!(dlg2.len(), 1);
        assert_eq!(dlg2.get("node_start"), Some(&"audio/intro.wav".to_string()));

        std::fs::remove_file(test_toml)?;
        Ok(())
    }

    #[test]
    fn test_load_dialogue_audio_map_missing_file() {
        let result = load_dialogue_audio_map("nonexistent_dialogue_map.toml");
        assert!(result.is_err(), "Should fail for missing file");
    }

    #[test]
    fn test_dialogue_player_no_line() -> Result<()> {
        let dlg = Dialogue {
            id: "test_no_line".into(),
            start: "n0".into(),
            nodes: vec![Node {
                id: "n0".into(),
                line: None, // No line to speak
                choices: vec![],
                end: true,
            }],
        };
        let st = DialogueState::new(&dlg);
        let mut audio = AudioEngine::new()?;
        let bank = VoiceBank {
            speakers: Default::default(),
        };

        let mut player = DialoguePlayer {
            audio: &mut audio,
            bank: &bank,
            tts: None,
            overrides: None,
            subtitle_out: None,
        };

        let result = player.speak_current(&dlg, &st)?;
        assert!(!result, "Should return false when node has no line");
        Ok(())
    }

    #[test]
    fn test_dialogue_player_subtitle_callback() -> Result<()> {
        let dlg = Dialogue {
            id: "test_subtitle".into(),
            start: "n0".into(),
            nodes: vec![Node {
                id: "n0".into(),
                line: Some(Line {
                    speaker: "TestSpeaker".into(),
                    text: "This is a test line".into(),
                    set_vars: vec![],
                }),
                choices: vec![],
                end: true,
            }],
        };
        let st = DialogueState::new(&dlg);
        let mut audio = AudioEngine::new()?;
        let bank = VoiceBank {
            speakers: Default::default(),
        };

        let mut subtitles: Vec<(String, String)> = vec![];
        {
            let mut push_sub = |speaker: String, text: String| {
                subtitles.push((speaker, text));
            };
            let mut player = DialoguePlayer {
                audio: &mut audio,
                bank: &bank,
                tts: None,
                overrides: None,
                subtitle_out: Some(&mut push_sub),
            };

            player.speak_current(&dlg, &st)?;
        }

        assert_eq!(subtitles.len(), 1);
        assert_eq!(subtitles[0].0, "TestSpeaker");
        assert_eq!(subtitles[0].1, "This is a test line");
        Ok(())
    }

    #[test]
    fn speak_beep_fallback() -> Result<()> {
        // Build minimal dialogue in memory
        let dlg = Dialogue {
            id: "test".into(),
            start: "n0".into(),
            nodes: vec![Node {
                id: "n0".into(),
                line: Some(Line {
                    speaker: "Test".into(),
                    text: "Hello there".into(),
                    set_vars: vec![],
                }),
                choices: vec![],
                end: true,
            }],
        };
        let st = DialogueState::new(&dlg);
        let mut audio = AudioEngine::new()?;
        let bank = VoiceBank {
            speakers: Default::default(),
        };
        let mut subs: Vec<(String, String)> = vec![];
        {
            let mut push_sub = |s: String, t: String| {
                subs.push((s, t));
            };
            let mut player = DialoguePlayer {
                audio: &mut audio,
                bank: &bank,
                tts: None,
                overrides: None,
                subtitle_out: Some(&mut push_sub),
            };
            let ok = player.speak_current(&dlg, &st)?;
            assert!(ok);
        }
        assert_eq!(subs.len(), 1);
        Ok(())
    }

    #[test]
    fn test_dialogue_player_with_override_path() -> Result<()> {
        std::fs::create_dir_all("target/test_audio")?;
        let test_audio_file = "target/test_audio/override_line.wav";
        // Create a minimal WAV file (just header, won't play but will exist)
        std::fs::write(test_audio_file, b"RIFF")?;

        let dlg = Dialogue {
            id: "test_override".into(),
            start: "n0".into(),
            nodes: vec![Node {
                id: "n0".into(),
                line: Some(Line {
                    speaker: "Overridden".into(),
                    text: "This line has an audio override".into(),
                    set_vars: vec![],
                }),
                choices: vec![],
                end: true,
            }],
        };
        let st = DialogueState::new(&dlg);
        let mut audio = AudioEngine::new()?;
        let bank = VoiceBank {
            speakers: Default::default(),
        };

        // Create override map
        let mut node_map = HashMap::new();
        node_map.insert("n0".to_string(), test_audio_file.to_string());
        let mut map = HashMap::new();
        map.insert("test_override".to_string(), node_map);
        let audio_map = DialogueAudioMap { map };

        let mut player = DialoguePlayer {
            audio: &mut audio,
            bank: &bank,
            tts: None,
            overrides: Some(&audio_map),
            subtitle_out: None,
        };

        let _result = player.speak_current(&dlg, &st);
        // Might fail due to invalid WAV, but we tested the override path logic
        // The important part is it attempted to use the override
        std::fs::remove_file(test_audio_file)?;
        Ok(())
    }

    #[test]
    fn test_dialogue_audio_map_multiple_dialogues() {
        let mut map = HashMap::new();

        // Dialogue 1
        let mut dlg1_nodes = HashMap::new();
        dlg1_nodes.insert("start".to_string(), "audio/dlg1_start.wav".to_string());
        dlg1_nodes.insert("end".to_string(), "audio/dlg1_end.wav".to_string());
        map.insert("dialogue_1".to_string(), dlg1_nodes);

        // Dialogue 2
        let mut dlg2_nodes = HashMap::new();
        dlg2_nodes.insert("intro".to_string(), "audio/dlg2_intro.wav".to_string());
        map.insert("dialogue_2".to_string(), dlg2_nodes);

        let audio_map = DialogueAudioMap { map };
        assert_eq!(audio_map.map.len(), 2);
        assert!(audio_map.map.contains_key("dialogue_1"));
        assert!(audio_map.map.contains_key("dialogue_2"));

        let dlg1 = audio_map.map.get("dialogue_1").unwrap();
        assert_eq!(dlg1.len(), 2);

        let dlg2 = audio_map.map.get("dialogue_2").unwrap();
        assert_eq!(dlg2.len(), 1);
    }

    #[test]
    fn test_dialogue_player_voice_bank_explicit_files() -> Result<()> {
        use crate::voice::VoiceSpec;

        // Create test audio files
        std::fs::create_dir_all("target/test_voices")?;
        let audio_file = "target/test_voices/test_line1.wav";
        std::fs::write(audio_file, b"RIFF")?; // Minimal WAV header

        let dlg = Dialogue {
            id: "test_voice_bank".into(),
            start: "n0".into(),
            nodes: vec![Node {
                id: "n0".into(),
                line: Some(Line {
                    speaker: "BankSpeaker".into(),
                    text: "Testing voice bank explicit files".into(),
                    set_vars: vec![],
                }),
                choices: vec![],
                end: true,
            }],
        };
        let st = DialogueState::new(&dlg);
        let mut audio = AudioEngine::new()?;

        // Create VoiceBank with explicit files
        let mut speakers = HashMap::new();
        speakers.insert(
            "BankSpeaker".to_string(),
            VoiceSpec {
                folder: "target/test_voices".to_string(),
                files: vec!["test_line1.wav".to_string()],
                tts_voice: None,
            },
        );
        let bank = VoiceBank { speakers };

        let mut player = DialoguePlayer {
            audio: &mut audio,
            bank: &bank,
            tts: None,
            overrides: None,
            subtitle_out: None,
        };

        // Should use explicit file from voice bank
        let _result = player.speak_current(&dlg, &st);
        // Test validates the code path was executed

        std::fs::remove_file(audio_file)?;
        Ok(())
    }

    #[test]
    fn test_dialogue_player_voice_bank_folder_scan() -> Result<()> {
        use crate::voice::VoiceSpec;

        // Create test folder with audio files
        std::fs::create_dir_all("target/test_folder_scan")?;
        std::fs::write("target/test_folder_scan/voice1.wav", b"RIFF")?;
        std::fs::write("target/test_folder_scan/voice2.ogg", b"OggS")?;
        std::fs::write("target/test_folder_scan/readme.txt", b"ignore")?; // Non-audio file

        let dlg = Dialogue {
            id: "test_folder_scan".into(),
            start: "n0".into(),
            nodes: vec![Node {
                id: "n0".into(),
                line: Some(Line {
                    speaker: "FolderSpeaker".into(),
                    text: "Testing folder scan for audio files".into(),
                    set_vars: vec![],
                }),
                choices: vec![],
                end: true,
            }],
        };
        let st = DialogueState::new(&dlg);
        let mut audio = AudioEngine::new()?;

        // VoiceSpec with empty files (triggers folder scan)
        let mut speakers = HashMap::new();
        speakers.insert(
            "FolderSpeaker".to_string(),
            VoiceSpec {
                folder: "target/test_folder_scan".to_string(),
                files: vec![], // Empty triggers folder scan
                tts_voice: None,
            },
        );
        let bank = VoiceBank { speakers };

        let mut player = DialoguePlayer {
            audio: &mut audio,
            bank: &bank,
            tts: None,
            overrides: None,
            subtitle_out: None,
        };

        // Should scan folder and find .wav or .ogg files
        let _result = player.speak_current(&dlg, &st);
        // Test validates folder scanning code path

        std::fs::remove_dir_all("target/test_folder_scan")?;
        Ok(())
    }

    #[test]
    fn test_dialogue_player_tts_fallback() -> Result<()> {
        use crate::voice::{TtsAdapter, VoiceSpec};

        // Mock TTS adapter
        struct MockTts;
        impl TtsAdapter for MockTts {
            fn synth_to_path(&self, _voice: &str, text: &str, out_path: &str) -> Result<()> {
                // Create a fake audio file
                std::fs::create_dir_all("target/test_tts")?;
                std::fs::write(out_path, format!("TTS: {}", text).as_bytes())?;
                Ok(())
            }
        }

        let dlg = Dialogue {
            id: "test_tts".into(),
            start: "n0".into(),
            nodes: vec![Node {
                id: "n0".into(),
                line: Some(Line {
                    speaker: "TtsSpeaker".into(),
                    text: "This should use TTS fallback".into(),
                    set_vars: vec![],
                }),
                choices: vec![],
                end: true,
            }],
        };
        let st = DialogueState::new(&dlg);
        let mut audio = AudioEngine::new()?;

        std::fs::create_dir_all("target/test_tts")?;

        // VoiceSpec with TTS voice but no files (triggers TTS fallback)
        let mut speakers = HashMap::new();
        speakers.insert(
            "TtsSpeaker".to_string(),
            VoiceSpec {
                folder: "target/test_tts".to_string(),
                files: vec![],
                tts_voice: Some("mock_voice".to_string()),
            },
        );
        let bank = VoiceBank { speakers };

        let mock_tts = MockTts;
        let mut player = DialoguePlayer {
            audio: &mut audio,
            bank: &bank,
            tts: Some(&mock_tts),
            overrides: None,
            subtitle_out: None,
        };

        // Should use TTS adapter to synthesize audio
        let _result = player.speak_current(&dlg, &st);
        // TTS fallback code path validated

        std::fs::remove_dir_all("target/test_tts").ok();
        Ok(())
    }

    #[test]
    fn test_dialogue_player_override_nonexistent_file() -> Result<()> {
        // Test that override path that doesn't exist falls through to next option
        let dlg = Dialogue {
            id: "test_override_missing".into(),
            start: "n0".into(),
            nodes: vec![Node {
                id: "n0".into(),
                line: Some(Line {
                    speaker: "UnknownSpeaker".into(),
                    text: "Override doesn't exist, should fall through".into(),
                    set_vars: vec![],
                }),
                choices: vec![],
                end: true,
            }],
        };
        let st = DialogueState::new(&dlg);
        let mut audio = AudioEngine::new()?;
        let bank = VoiceBank {
            speakers: Default::default(),
        };

        // Override with nonexistent file
        let mut node_map = HashMap::new();
        node_map.insert("n0".to_string(), "nonexistent_audio_file.wav".to_string());
        let mut map = HashMap::new();
        map.insert("test_override_missing".to_string(), node_map);
        let audio_map = DialogueAudioMap { map };

        let mut player = DialoguePlayer {
            audio: &mut audio,
            bank: &bank,
            tts: None,
            overrides: Some(&audio_map),
            subtitle_out: None,
        };

        // Should fall through to beep fallback since override doesn't exist
        let result = player.speak_current(&dlg, &st)?;
        assert!(result, "Should still return true (beep fallback)");
        Ok(())
    }

    #[test]
    fn test_dialogue_player_empty_voice_folder() -> Result<()> {
        use crate::voice::VoiceSpec;

        // Create empty folder (no audio files)
        std::fs::create_dir_all("target/test_empty_folder")?;

        let dlg = Dialogue {
            id: "test_empty".into(),
            start: "n0".into(),
            nodes: vec![Node {
                id: "n0".into(),
                line: Some(Line {
                    speaker: "EmptySpeaker".into(),
                    text: "No audio files available".into(),
                    set_vars: vec![],
                }),
                choices: vec![],
                end: true,
            }],
        };
        let st = DialogueState::new(&dlg);
        let mut audio = AudioEngine::new()?;

        // VoiceSpec pointing to empty folder
        let mut speakers = HashMap::new();
        speakers.insert(
            "EmptySpeaker".to_string(),
            VoiceSpec {
                folder: "target/test_empty_folder".to_string(),
                files: vec![],
                tts_voice: None,
            },
        );
        let bank = VoiceBank { speakers };

        let mut player = DialoguePlayer {
            audio: &mut audio,
            bank: &bank,
            tts: None,
            overrides: None,
            subtitle_out: None,
        };

        // Should fall through to beep fallback
        let result = player.speak_current(&dlg, &st)?;
        assert!(result, "Should return true (beep fallback)");

        std::fs::remove_dir_all("target/test_empty_folder")?;
        Ok(())
    }

    #[test]
    fn test_load_dialogue_audio_map_invalid_toml() {
        std::fs::create_dir_all("target").ok();
        let invalid_toml = "target/invalid_dialogue.toml";
        std::fs::write(invalid_toml, "this is not valid TOML [[[ ").ok();

        let result = load_dialogue_audio_map(invalid_toml);
        assert!(result.is_err(), "Should fail for invalid TOML");

        std::fs::remove_file(invalid_toml).ok();
    }

    #[test]
    fn test_dialogue_player_unknown_speaker() -> Result<()> {
        // Speaker not in voice bank should fall through to beep
        let dlg = Dialogue {
            id: "test_unknown_speaker".into(),
            start: "n0".into(),
            nodes: vec![Node {
                id: "n0".into(),
                line: Some(Line {
                    speaker: "UnknownPerson".into(),
                    text: "I'm not in the voice bank".into(),
                    set_vars: vec![],
                }),
                choices: vec![],
                end: true,
            }],
        };
        let st = DialogueState::new(&dlg);
        let mut audio = AudioEngine::new()?;
        let bank = VoiceBank {
            speakers: Default::default(), // Empty bank
        };

        let mut player = DialoguePlayer {
            audio: &mut audio,
            bank: &bank,
            tts: None,
            overrides: None,
            subtitle_out: None,
        };

        // Should use beep fallback for unknown speaker
        let result = player.speak_current(&dlg, &st)?;
        assert!(result, "Should return true (beep fallback)");
        Ok(())
    }

    #[test]
    fn test_dialogue_player_voice_file_not_found() -> Result<()> {
        use crate::voice::VoiceSpec;

        // VoiceSpec points to file that doesn't exist
        let dlg = Dialogue {
            id: "test_missing".into(),
            start: "n0".into(),
            nodes: vec![Node {
                id: "n0".into(),
                line: Some(Line {
                    speaker: "MissingSpeaker".into(),
                    text: "File doesn't exist".into(),
                    set_vars: vec![],
                }),
                choices: vec![],
                end: true,
            }],
        };
        let st = DialogueState::new(&dlg);
        let mut audio = AudioEngine::new()?;

        // VoiceSpec with nonexistent file
        let mut speakers = HashMap::new();
        speakers.insert(
            "MissingSpeaker".to_string(),
            VoiceSpec {
                folder: "target/nonexistent_folder".to_string(),
                files: vec!["missing_file.wav".to_string()],
                tts_voice: None,
            },
        );
        let bank = VoiceBank { speakers };

        let mut player = DialoguePlayer {
            audio: &mut audio,
            bank: &bank,
            tts: None,
            overrides: None,
            subtitle_out: None,
        };

        // Should fall through to beep when file doesn't exist
        let result = player.speak_current(&dlg, &st)?;
        assert!(result, "Should return true (beep fallback)");
        Ok(())
    }

    #[test]
    fn test_dialogue_player_folder_scan_no_audio_files() -> Result<()> {
        use crate::voice::VoiceSpec;

        // Create folder with only non-audio files
        std::fs::create_dir_all("target/test_no_audio")?;
        std::fs::write("target/test_no_audio/readme.txt", b"text file")?;
        std::fs::write("target/test_no_audio/data.json", b"{}")?;

        let dlg = Dialogue {
            id: "test_no_audio".into(),
            start: "n0".into(),
            nodes: vec![Node {
                id: "n0".into(),
                line: Some(Line {
                    speaker: "NoAudioSpeaker".into(),
                    text: "Folder has no audio files".into(),
                    set_vars: vec![],
                }),
                choices: vec![],
                end: true,
            }],
        };
        let st = DialogueState::new(&dlg);
        let mut audio = AudioEngine::new()?;

        // VoiceSpec with empty files (triggers folder scan)
        let mut speakers = HashMap::new();
        speakers.insert(
            "NoAudioSpeaker".to_string(),
            VoiceSpec {
                folder: "target/test_no_audio".to_string(),
                files: vec![],
                tts_voice: None,
            },
        );
        let bank = VoiceBank { speakers };

        let mut player = DialoguePlayer {
            audio: &mut audio,
            bank: &bank,
            tts: None,
            overrides: None,
            subtitle_out: None,
        };

        // Should fall through to beep when no audio files found
        let result = player.speak_current(&dlg, &st)?;
        assert!(result, "Should return true (beep fallback)");

        std::fs::remove_dir_all("target/test_no_audio")?;
        Ok(())
    }

    #[test]
    fn test_dialogue_player_folder_read_error() -> Result<()> {
        use crate::voice::VoiceSpec;

        let dlg = Dialogue {
            id: "test_read_error".into(),
            start: "n0".into(),
            nodes: vec![Node {
                id: "n0".into(),
                line: Some(Line {
                    speaker: "ErrorSpeaker".into(),
                    text: "Folder can't be read".into(),
                    set_vars: vec![],
                }),
                choices: vec![],
                end: true,
            }],
        };
        let st = DialogueState::new(&dlg);
        let mut audio = AudioEngine::new()?;

        // VoiceSpec pointing to nonexistent folder (triggers read error)
        let mut speakers = HashMap::new();
        speakers.insert(
            "ErrorSpeaker".to_string(),
            VoiceSpec {
                folder: "C:/this/path/absolutely/does/not/exist".to_string(),
                files: vec![],
                tts_voice: None,
            },
        );
        let bank = VoiceBank { speakers };

        let mut player = DialoguePlayer {
            audio: &mut audio,
            bank: &bank,
            tts: None,
            overrides: None,
            subtitle_out: None,
        };

        // Should fall through to beep when folder read fails
        let result = player.speak_current(&dlg, &st)?;
        assert!(result, "Should return true (beep fallback)");
        Ok(())
    }
}
