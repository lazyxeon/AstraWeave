# Audio Spatial Demo Walkthrough

The `audio_spatial_demo` example demonstrates AstraWeave's **3D spatial audio system** - positional sound effects, music playback with crossfading, and listener tracking tied to the camera.

## Running the Example

```bash
cargo run -p audio_spatial_demo --release
```

> **Note**: This demo requires audio files in `assets/audio/`. If files are missing, you'll see error messages but the demo will still run for visualization.

## What It Demonstrates

- **3D positional audio**: Sounds panning based on listener position
- **Procedural beeps**: Generated tones at specified frequencies
- **Music playback**: Looped background music
- **Crossfading**: Smooth transitions between music tracks
- **Listener tracking**: Audio follows camera position/orientation

## Controls

| Key | Action |
|-----|--------|
| `W/A/S/D` | Move camera (and listener) |
| `Space` | Camera up |
| `Shift` | Camera down |
| Right-click + drag | Look around |
| `1` | Play beep at center (880 Hz) |
| `2` | Play beep at left (-3, 1, 0) at 660 Hz |
| `3` | Play beep at right (+3, 1, 0) at 440 Hz |
| `M` | Switch music track (crossfade) |
| `Esc` | Exit |

## Expected Behavior

1. **Launch**: Background music plays (if `assets/audio/bgm.ogg` exists)
2. **Press 1**: High-pitched beep plays from center
3. **Press 2**: Medium-pitched beep plays from left (pans left in headphones)
4. **Press 3**: Low-pitched beep plays from right (pans right)
5. **Move camera with W/A/S/D**: Audio panning shifts as listener moves
6. **Press M**: Music crossfades to alternate track

## Code Walkthrough

### 1. Audio Engine Setup

```rust
let mut audio = AudioEngine::new()?;
audio.set_master_volume(1.0);

// Try to play looped background music
let _ = audio.play_music(
    MusicTrack {
        path: "assets/audio/bgm.ogg".into(),
        looped: true,
    },
    1.0,  // Volume
);
```

The `AudioEngine`:
- Initializes the audio device (rodio backend)
- Manages multiple simultaneous sounds
- Controls master/music/SFX volume buses

### 2. 3D Positional Beeps

```rust
// Center beep (880 Hz, 0.25s duration, 0.5 volume)
self.audio.play_sfx_3d_beep(
    100,                    // Emitter ID
    vec3(0.0, 1.0, 0.0),   // Position (center)
    880.0,                  // Frequency (Hz)
    0.25,                   // Duration (seconds)
    0.5,                    // Volume
);

// Left beep (660 Hz)
self.audio.play_sfx_3d_beep(
    101,
    vec3(-3.0, 1.0, 0.0),  // Left of center
    660.0,
    0.25,
    0.5,
);

// Right beep (440 Hz)
self.audio.play_sfx_3d_beep(
    102,
    vec3(3.0, 1.0, 0.0),   // Right of center
    440.0,
    0.25,
    0.5,
);
```

The emitter ID allows tracking/stopping specific sounds.

### 3. Music Crossfading

```rust
self.audio.play_music(
    MusicTrack {
        path: "assets/audio/bgm_alt.ogg".into(),
        looped: true,
    },
    1.25,  // Crossfade duration (seconds)
);
```

When switching tracks:
1. Current music fades out over 1.25 seconds
2. New music fades in simultaneously
3. No audio gap or jarring transition

### 4. Listener Updates

```rust
// Calculate forward direction from camera yaw/pitch
let forward = glam::Quat::from_euler(
    glam::EulerRot::YXZ,
    self.camera.yaw,
    self.camera.pitch,
    0.0,
) * vec3(0.0, 0.0, -1.0);

// Update audio listener pose
self.audio.update_listener(ListenerPose {
    position: self.camera.position,
    forward,
    up: vec3(0.0, 1.0, 0.0),
});

// Advance audio processing
self.audio.tick(dt);
```

The listener pose determines:
- **Position**: Where sounds are heard from
- **Forward**: Which direction is "front"
- **Up**: Which direction is "up" (for proper panning)

## Audio Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    AudioEngine                          │
├─────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────┐ │
│  │ Music Bus   │  │ SFX Bus     │  │ Voice Bus       │ │
│  │ (looped)    │  │ (one-shot)  │  │ (dialogue)      │ │
│  └─────────────┘  └─────────────┘  └─────────────────┘ │
│                        ↓                                │
│              ┌─────────────────┐                        │
│              │ Spatial Mixer   │ ← ListenerPose         │
│              │ (3D panning)    │                        │
│              └─────────────────┘                        │
│                        ↓                                │
│              ┌─────────────────┐                        │
│              │ Master Volume   │                        │
│              └─────────────────┘                        │
│                        ↓                                │
│              ┌─────────────────┐                        │
│              │ rodio Output    │                        │
│              └─────────────────┘                        │
│                                                         │
│  Backend: rodio 0.17                                    │
└─────────────────────────────────────────────────────────┘
```

## ListenerPose

```rust
pub struct ListenerPose {
    pub position: Vec3,  // World-space position
    pub forward: Vec3,   // Look direction (normalized)
    pub up: Vec3,        // Up vector (typically Y-up)
}
```

The audio system uses this to compute:
- **Attenuation**: Sounds get quieter with distance
- **Panning**: Left/right based on angle to listener
- **Doppler** (optional): Pitch shift for moving sources

## Volume Buses

```rust
// Individual bus control
audio.set_master_volume(1.0);   // Overall volume
audio.set_music_volume(0.8);    // Background music
audio.set_sfx_volume(1.0);      // Sound effects
audio.set_voice_volume(1.0);    // Dialogue/narration
```

## Supported Formats

AstraWeave audio supports:
- **OGG Vorbis** (recommended for music)
- **WAV** (uncompressed, good for short SFX)
- **MP3** (widely compatible)
- **FLAC** (lossless, large files)

## Performance Notes

- **Simultaneous sounds**: Up to 64 by default
- **Streaming**: Large files (>1MB) stream from disk
- **CPU usage**: <1% for typical game audio
- **Memory**: ~10KB per short sound, streaming for long tracks

## Related Examples

- [Physics Demo](./physics-demo.md) - 3D environment
- [Unified Showcase](./unified-showcase.md) - Visual rendering
- [Hello Companion](./hello-companion.md) - AI with audio cues

## Troubleshooting

### No audio output
- Check system audio settings
- Verify audio device is connected
- Try different audio files

### Crackling/popping
- Increase audio buffer size
- Reduce simultaneous sound count
- Check for CPU throttling

### Sounds don't pan correctly
- Verify `update_listener()` is called each frame
- Check that forward/up vectors are normalized
- Ensure sound positions are in world space

### Missing audio files
The demo will print errors if `assets/audio/bgm.ogg` is missing. Create the directory and add audio files, or the demo will run silently.

## Source Location

- **Example**: `examples/audio_spatial_demo/src/main.rs` (215 lines)
- **Audio Engine**: `astraweave-audio/src/engine.rs`
- **Spatial Mixer**: `astraweave-audio/src/spatial.rs`
