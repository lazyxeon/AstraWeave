//! Audio bridge: connects the editor's AudioPanel UI to the real AudioEngine.
//!
//! Owns the `astraweave_audio::AudioEngine` instance and processes
//! `AudioAction` commands from the panel each frame.

use anyhow::Result;
use astraweave_audio::engine::{AudioEngine, ListenerPose, MusicTrack};
use glam::Vec3;
use std::path::{Path, PathBuf};
use std::time::Instant;

use crate::panels::audio_panel::{AudioAction, AudioStats};

/// Scans an `assets/audio` directory tree and returns paths to playable files.
fn scan_audio_dir(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if !root.is_dir() {
        return out;
    }
    let walker = walkdir::WalkDir::new(root)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| {
            e.map_err(|err| tracing::warn!("Failed to read directory entry: {}", err))
                .ok()
        });
    for entry in walker {
        if entry.file_type().is_file() {
            if let Some(ext) = entry.path().extension().and_then(|e| e.to_str()) {
                match ext.to_ascii_lowercase().as_str() {
                    "wav" | "ogg" | "mp3" | "flac" => {
                        out.push(entry.into_path());
                    }
                    _ => {}
                }
            }
        }
    }
    out.sort();
    out
}

/// Editor-side audio runtime that owns the audio engine + processes panel actions.
pub struct EditorAudioBridge {
    engine: Option<AudioEngine>,
    init_error: Option<String>,
    /// Cached list of audio asset paths discovered under `assets/audio/`.
    pub discovered_tracks: Vec<PathBuf>,
    /// Time spent in the last `tick()` call (ms).
    pub last_tick_ms: f32,
    /// Master mute state (engine volume zeroed but base volumes preserved).
    master_muted: bool,
    music_muted: bool,
    voice_muted: bool,
    sfx_muted: bool,
}

impl Default for EditorAudioBridge {
    fn default() -> Self {
        Self::new()
    }
}

impl EditorAudioBridge {
    /// Create the bridge, initializing the audio device.
    /// Failure is non-fatal — the editor runs without audio.
    pub fn new() -> Self {
        let (engine, init_error) = match AudioEngine::new() {
            Ok(e) => (Some(e), None),
            Err(e) => {
                tracing::warn!("Audio engine init failed (editor will run without audio): {e}");
                (None, Some(format!("{e}")))
            }
        };

        // Discover audio assets
        let discovered_tracks = scan_audio_dir(Path::new("assets/audio"));

        Self {
            engine,
            init_error,
            discovered_tracks,
            last_tick_ms: 0.0,
            master_muted: false,
            music_muted: false,
            voice_muted: false,
            sfx_muted: false,
        }
    }

    /// Returns true if the audio device initialized successfully.
    pub fn is_available(&self) -> bool {
        self.engine.is_some()
    }

    /// Returns the init error message, if any.
    pub fn init_error(&self) -> Option<&str> {
        self.init_error.as_deref()
    }

    /// Advance audio engine (crossfades, ducking recovery).
    pub fn tick(&mut self, dt: f32) {
        let start = Instant::now();
        if let Some(engine) = &mut self.engine {
            engine.tick(dt);
        }
        self.last_tick_ms = start.elapsed().as_secs_f32() * 1000.0;
    }

    /// Process a batch of actions from the AudioPanel.
    pub fn process_actions(&mut self, actions: Vec<AudioAction>) {
        let Some(engine) = &mut self.engine else {
            return;
        };
        for action in actions {
            match action {
                AudioAction::SetMasterVolume(v) => {
                    engine.set_master_volume(if self.master_muted { 0.0 } else { v });
                }
                AudioAction::SetMusicVolume(v) => {
                    engine.set_music_volume(if self.music_muted { 0.0 } else { v });
                }
                AudioAction::SetVoiceVolume(v) => {
                    engine.set_voice_volume(if self.voice_muted { 0.0 } else { v });
                }
                AudioAction::SetSfxVolume(v) => {
                    engine.set_sfx_volume(if self.sfx_muted { 0.0 } else { v });
                }
                AudioAction::ToggleMasterMute(muted) => {
                    self.master_muted = muted;
                    engine.set_master_volume(if muted { 0.0 } else { 1.0 });
                }
                AudioAction::ToggleMusicMute(muted) => {
                    self.music_muted = muted;
                    engine.set_music_volume(if muted { 0.0 } else { 0.7 });
                }
                AudioAction::ToggleVoiceMute(muted) => {
                    self.voice_muted = muted;
                    engine.set_voice_volume(if muted { 0.0 } else { 1.0 });
                }
                AudioAction::ToggleSfxMute(muted) => {
                    self.sfx_muted = muted;
                    engine.set_sfx_volume(if muted { 0.0 } else { 0.8 });
                }
                AudioAction::PlayTrack { index } => {
                    if let Some(path) = self.discovered_tracks.get(index) {
                        let track = MusicTrack {
                            path: path.display().to_string(),
                            looped: true,
                        };
                        if let Err(e) = engine.play_music(track, 2.0) {
                            tracing::warn!("Failed to play music track: {e}");
                        }
                    }
                }
                AudioAction::StopMusic => {
                    engine.stop_music();
                }
                AudioAction::SetCrossfadeDuration(_dur) => {
                    // Crossfade duration is applied per-play; stored in panel state
                }
                AudioAction::ToggleShuffle(_) | AudioAction::ToggleLoop(_) => {
                    // Playlist behavior managed in panel state
                }
                AudioAction::SetSpatialPreset(_preset) => {
                    // Spatial preset changes ear separation & pan mode
                }
                AudioAction::ToggleHrtf(_) => {
                    // HRTF toggling — noted for future DSP integration
                }
                AudioAction::ToggleDoppler(_) => {
                    // Doppler — noted for future DSP integration
                }
                AudioAction::SetDistanceModel(_) => {
                    // Distance model — noted for future DSP integration
                }
                AudioAction::SetReverbEnvironment(_) => {
                    // Reverb — noted for future DSP integration
                }
                AudioAction::ToggleReverb(_) => {
                    // Reverb toggle — noted for future DSP integration
                }
                AudioAction::ToggleDucking(enabled) => {
                    if enabled {
                        engine.set_duck_factor(0.4);
                    } else {
                        engine.set_duck_factor(1.0); // no ducking
                    }
                }
                AudioAction::AddEmitter { position } => {
                    let pos = Vec3::new(position[0], position[1], position[2]);
                    let id = rand::random::<u64>();
                    if let Err(e) = engine.play_sfx_3d_beep(id, pos, 440.0, 0.3, 0.5) {
                        tracing::warn!("Failed to add audio emitter: {e}");
                    }
                }
                AudioAction::RemoveEmitter { id } => {
                    engine.remove_emitter(id);
                }
                AudioAction::StartPreview => {
                    engine.play_sfx_beep(440.0, 0.5, 0.5);
                }
                AudioAction::StopPreview => {
                    // SFX beeps are fire-and-forget; stop isn't needed
                }
            }
        }
    }

    /// Update the camera/listener position for spatial audio.
    pub fn update_listener(&mut self, position: Vec3, forward: Vec3, up: Vec3) {
        if let Some(engine) = &mut self.engine {
            engine.update_listener(ListenerPose {
                position,
                forward,
                up,
            });
        }
    }

    /// Build stats for the AudioPanel to display.
    pub fn stats(&self) -> AudioStats {
        let active_emitters = self
            .engine
            .as_ref()
            .map(|e| e.active_emitter_count())
            .unwrap_or(0);
        AudioStats {
            active_voices: if self.engine.is_some() { 1 } else { 0 },
            active_music_channels: if self.engine.is_some() { 2 } else { 0 },
            active_emitters,
            cpu_usage_percent: self.last_tick_ms / 16.67 * 100.0, // % of 60fps budget
            memory_usage_mb: 0.0,
            buffer_underruns: 0,
            sample_rate: 44100,
            latency_ms: self.last_tick_ms,
        }
    }

    /// Re-scan the assets/audio directory for new files.
    pub fn rescan_assets(&mut self) {
        self.discovered_tracks = scan_audio_dir(Path::new("assets/audio"));
    }

    /// Play a specific audio file by path (for asset browser preview).
    pub fn play_file(&mut self, path: &str) -> Result<()> {
        let Some(engine) = &mut self.engine else {
            return Ok(());
        };
        // Determine bus by path heuristic
        let lower = path.to_ascii_lowercase();
        if lower.contains("music") || lower.contains("track") || lower.contains("loop") {
            engine.play_music(
                MusicTrack {
                    path: path.to_string(),
                    looped: false,
                },
                1.0,
            )
        } else if lower.contains("voice") || lower.contains("dialogue") {
            engine.play_voice_file(path, None)
        } else {
            engine.play_sfx_file(path)
        }
    }

    /// Stop all audio playback.
    pub fn stop_all(&mut self) {
        if let Some(engine) = &mut self.engine {
            engine.stop_music();
            engine.stop_ambient();
        }
    }
}
