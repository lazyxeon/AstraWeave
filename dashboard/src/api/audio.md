# Audio API Reference

> **Crate**: `astraweave-audio`  
> **Coverage**: ~72%  
> **Tests**: 150+

Spatial audio with mixer buses, crossfading, reverb zones, and audio occlusion.

## Quick Links

- [rustdoc](https://docs.rs/astraweave-audio) (when published)
- [Source](https://github.com/astraweave/astraweave/tree/main/astraweave-audio)
- [Audio Guide](../core-systems/audio.md)

---

## Core Types

### AudioEngine

Central audio system with mixer.

```rust
use astraweave_audio::{AudioEngine, AudioConfig};

let config = AudioConfig {
    sample_rate: 48000,
    buffer_size: 512,
    ..Default::default()
};

let mut audio = AudioEngine::new(config)?;

// Play sound
audio.play("sfx/explosion.ogg", PlayConfig::default());

// Update each frame
audio.update(listener_position, listener_orientation);
```

---

### AudioMixer

4-bus mixing system.

```rust
use astraweave_audio::mixer::{AudioMixer, Bus};

let mut mixer = AudioMixer::new();

// Configure bus volumes
mixer.set_volume(Bus::Master, 1.0);
mixer.set_volume(Bus::Music, 0.8);
mixer.set_volume(Bus::Sfx, 1.0);
mixer.set_volume(Bus::Voice, 1.0);

// Mute bus
mixer.set_muted(Bus::Music, true);
```

---

### SpatialSource

3D positioned audio source.

```rust
use astraweave_audio::SpatialSource;

let mut source = SpatialSource::new("ambient/wind.ogg");
source.set_position(Vec3::new(10.0, 0.0, 5.0));
source.set_looping(true);
source.set_min_distance(1.0);
source.set_max_distance(50.0);
source.play();
```

---

### MusicLayer

Dynamic music with layers and crossfading.

```rust
use astraweave_audio::music::{MusicPlayer, MusicLayer};

let mut music = MusicPlayer::new();

// Add layers
music.add_layer(MusicLayer::new("music/ambient.ogg"));
music.add_layer(MusicLayer::new("music/combat.ogg"));
music.add_layer(MusicLayer::new("music/boss.ogg"));

// Crossfade between layers
music.crossfade_to(1, Duration::from_secs(2)); // Combat music
```

---

### ReverbZone

Environment-based reverb.

```rust
use astraweave_audio::reverb::{ReverbZone, ReverbPreset};

let zone = ReverbZone::new(ReverbPreset::Cave)
    .with_bounds(aabb)
    .with_blend_distance(5.0);

// Add to audio system
audio.add_reverb_zone(zone);
```

**Presets**: `Small`, `Medium`, `Large`, `Hall`, `Cave`, `Outdoor`, `Underwater`

---

### AudioOcclusion

Raycast-based sound occlusion.

```rust
use astraweave_audio::occlusion::AudioOcclusion;

let mut occlusion = AudioOcclusion::new(&physics);

// Update occlusion for all sources
occlusion.update(listener_position, &sources);

// Apply to source
let factor = occlusion.get_factor(source_id);
source.set_volume(base_volume * factor);
```

---

## Performance

| Operation | Latency | Notes |
|-----------|---------|-------|
| Source play | ~100 µs | Initial setup |
| Mixer update | ~50 µs | Per frame |
| Spatial update | ~1 µs | Per source |
| Occlusion raycast | ~5 µs | Per source |

---

## See Also

- [Audio Guide](../core-systems/audio.md)
- [Cinematics Integration](./cinematics.md#audio-tracks)
