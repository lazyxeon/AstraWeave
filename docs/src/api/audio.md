# Audio API Reference

> **Crate**: `astraweave-audio`  
> **Manifest version**: `0.1.0`

Public modules are `engine`, `voice`, and `dialogue_runtime`. The crate root re-exports `AudioEngine`, `EmitterId`, `ListenerPose`, `MusicTrack`, `PanMode`, `DialogueAudioMap`, `DialoguePlayer`, `VoiceBank`, `VoiceSpec`, `TtsAdapter`, and `SimpleSineTts`.

There is no `AudioConfig`, `AudioMixer`, `Bus`, `SpatialSource`, `MusicLayer`, `ReverbZone`, `AudioOcclusion`, or `PlayConfig` type.

## AudioEngine

`AudioEngine::new` and `AudioEngine::null` return `anyhow::Result<Self>`. `null` keeps mixer state and does not open an OS device. Playback methods on that backend drop samples.

```rust
use astraweave_audio::{AudioEngine, ListenerPose, MusicTrack};
use glam::vec3;

fn open() -> anyhow::Result<()> {
    let mut audio = AudioEngine::null()?;
    audio.set_master_volume(1.0);
    audio.set_music_volume(0.8);
    audio.set_sfx_volume(1.0);
    audio.set_voice_volume(1.0);
    audio.update_listener(ListenerPose {
        position: vec3(0.0, 0.0, 0.0),
        forward: vec3(1.0, 0.0, 0.0),
        up: vec3(0.0, 1.0, 0.0),
    });
    audio.tick(1.0 / 60.0);
    let track = MusicTrack {
        path: "music/theme.ogg".into(),
        looped: true,
    };
    audio.play_music(track, 1.0)?;
    Ok(())
}
```

`play_music` and `play_ambient` take `crossfade_sec: f32`. File playback is `play_sfx_file`, `play_voice_file`, and `play_sfx_3d_file`. `EmitterId` is `u64`.

`PanMode` is `StereoAngle` or `None`. `set_pan_mode` stores it.

## Voice and dialogue

`voice` exports `load_voice_bank`, `TtsAdapter`, `VoiceBank`, `VoiceSpec`, and `SimpleSineTts`. `dialogue_runtime` exports `load_dialogue_audio_map`, `DialogueAudioMap`, and `DialoguePlayer`.

The only feature in `Cargo.toml` is `mock_tts`.

## See Also

- [Audio Guide](../core-systems/audio.md)
- [Examples](../examples/audio-spatial.md)
