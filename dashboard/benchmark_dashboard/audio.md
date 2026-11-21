---
layout: default
title: Audio Subsystem
---

# Audio (astraweave-audio)

AstraWeave's audio engine provides spatial audio, a 5-bus mixer, crossfading, dialogue runtime, and TTS integration built on **rodio 0.17**. The crate has **239 tests**.

## Features

| Feature | Status |
|---------|--------|
| 5-bus mixer (Master/Music/SFX/Ambient/Voice) | ✅ |
| Per-bus volume and mute | ✅ |
| Music crossfading between tracks | ✅ |
| Ambient crossfading | ✅ |
| 3D spatial audio (position-based panning) | ✅ |
| Dialogue playback runtime | ✅ |
| Text-to-speech (TTS) adapter | ✅ |
| Voice bank system | ✅ |
| Mock TTS for testing | ✅ (feature `mock_tts`) |
| Audio occlusion (raycast) | 📋 Planned |
| Reverb zones | 📋 Planned |

## Mixer Architecture

```
Sound Sources
   │
   ├── Music Bus ────────┐  (crossfade pair for seamless transitions)
   ├── SFX Bus ──────────┤  (one-shot sound effects)
   ├── Ambient Bus ──────┤  (crossfade pair for environmental audio)
   ├── Voice Bus ────────┤  (dialogue and narration)
   └─────────────────────┤
                         │
                    Master Bus ──► Output
                    (global volume)
```

Each bus supports:
- Independent volume control (0.0–1.0)
- Individual volume fields: `master_volume`, `music_base_volume`, `sfx_base_volume`, `ambient_base_volume`, `voice_base_volume`

## Core Types

### AudioEngine

The central audio manager:

```rust
let mut audio = AudioEngine::new()?;

// Volume controls
audio.set_master_volume(0.8);

// Panning mode
audio.set_pan_mode(PanMode::Stereo);

// Update listener position each frame
audio.update_listener(listener_pose);
audio.tick(dt);
```

### Music Playback

```rust
// Play background music (crossfades automatically)
audio.play_music("exploration_theme.wav");
audio.stop_music();
```

### Ambient Audio

```rust
// Play ambient soundscape with crossfading
audio.play_ambient("forest_ambience.wav");
audio.set_ambient_volume(0.6);
audio.stop_ambient();

// Check crossfade status
if audio.is_ambient_crossfading() { /* still transitioning */ }
```

### Sound Effects

```rust
// 2D sound effect
audio.play_sfx_file("click.wav");
audio.play_sfx_beep(frequency, duration);

// 3D spatial sound effect
audio.play_sfx_3d_file(emitter_id, "explosion.wav", world_position);
audio.play_sfx_3d_beep(emitter_id, frequency, duration, world_position);
```

### Voice/Dialogue

```rust
// Play voice clip
audio.play_voice_file("npc_greeting.wav");
audio.play_voice_beep(frequency, duration);
```

## Dialogue Runtime

The dialogue system integrates the audio engine with the AI subsystem for NPC conversations:

```rust
use astraweave_audio::{DialoguePlayer, DialogueAudioMap, load_dialogue_audio_map};

// Load pre-mapped dialogue audio
let audio_map = load_dialogue_audio_map("dialogue_assets/")?;
let player = DialoguePlayer::new(audio_map);
```

Pipeline:
1. AI generates dialogue text via LLM or scripted dialogue trees
2. `DialoguePlayer` plays the corresponding voice clip or invokes TTS
3. UI displays subtitle text synced to audio playback
4. Player responses feed back into AI reasoning

## Voice System

```rust
use astraweave_audio::{VoiceBank, VoiceSpec, TtsAdapter, load_voice_bank};

// Load a voice bank for an NPC
let bank = load_voice_bank("npc_merchant/")?;

// TTS for dynamically generated dialogue (LLM output)
let tts = TtsAdapter::new(voice_spec);
```

The `SimpleSineTts` (feature `mock_tts`) provides a mock TTS implementation for testing without an actual TTS engine.

## Spatial Audio

The `EmitterId` (u64) system tracks 3D sound sources. Each frame:

1. `update_listener(pose)` updates the listener position/orientation
2. `tick(dt)` processes all active emitters
3. Panning is calculated based on emitter-to-listener vectors

`ListenerPose` carries position and orientation for realistic spatial audio rendering.

## Integration Points

- **UI**: Volume sliders in the Settings → Audio panel control all 5 buses
- **Physics**: Planned occlusion raycasts will use the physics world
- **AI**: Dialogue generation connects through the LLM persona system
- **Cinematics**: Timeline audio tracks trigger through the AudioEngine

[← Back to Home](index.html) · [Architecture](architecture.html) · [UI](ui.html)
