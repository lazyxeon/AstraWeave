# Anchor Audio File Specifications

**Project**: Veilweaver - Anchor System Audio  
**Date**: November 8, 2025  
**Purpose**: Technical specifications for 6 audio files required by the Anchor Audio System

---

## Overview

The Anchor Audio System requires **6 audio files** to provide immersive soundscapes for the loom node mechanics:
- **4 state-based hum variants** (Perfect, Stable, Unstable, Critical)
- **1 repair success sound** (5s restoration effect)
- **1 Echo pickup chime** (short reward feedback)

All files use **OGG Vorbis** format for compression and looping support.

---

## File 1: anchor_hum_perfect.ogg

**Status**: OPTIONAL (Currently not used - Perfect anchors are silent)  
**Path**: `assets/audio/anchor/anchor_hum_perfect.ogg`  
**Duration**: N/A (loop indefinitely if implemented)  
**Format**: OGG Vorbis, 44.1 kHz, mono, 128 kbps

**Audio Design**:
- **Frequency**: 440 Hz (A4 note, pure sine wave)
- **Timbre**: Clean, pristine, no harmonics
- **Volume**: 0% (currently silent in code, but file can be 100% for future use)
- **Loop**: Seamless (fade edges to avoid clicks)
- **Character**: Perfect reality stabilization, no distortion

**Implementation Note**: Currently, Perfect anchors (stability 1.0) are silent to emphasize their pristine state. This file is reserved for future design changes if we want to add a subtle "reality hum" to Perfect anchors.

---

## File 2: anchor_hum_stable.ogg

**Path**: `assets/audio/anchor/anchor_hum_stable.ogg`  
**Duration**: 5-10 seconds (loop seamlessly)  
**Format**: OGG Vorbis, 44.1 kHz, mono, 128 kbps

**Audio Design**:
- **Frequency**: 400-440 Hz (mostly A4, occasional flicker to lower harmonics)
- **Timbre**: Subtle buzz, occasional static pops (5% of duration)
- **Volume**: 20% relative to max
- **Loop**: Seamless with 0.1s crossfade at loop points
- **Flicker Pattern**: Rare (1-2 glitches per 5s loop)
- **Character**: Mostly stable with hints of reality strain

**Recommended Generation**:
```
Audacity/Reaper:
1. Generate 440 Hz sine wave (5s)
2. Add low-pass filter noise (50 Hz, -40 dB)
3. Insert 3-5 static clicks (0.01s each, random placement)
4. Fade in/out 0.1s at edges for seamless loop
5. Export as OGG Vorbis (Q6, 128 kbps)
```

**Reference**: Think "old fluorescent light hum" - mostly steady, occasional flicker.

---

## File 3: anchor_hum_unstable.ogg

**Path**: `assets/audio/anchor/anchor_hum_unstable.ogg`  
**Duration**: 3-5 seconds (loop seamlessly)  
**Format**: OGG Vorbis, 44.1 kHz, mono, 128 kbps

**Audio Design**:
- **Frequency**: 300-350 Hz (distorted, wavering pitch)
- **Timbre**: Heavy static, crackling, warble effect
- **Volume**: 50% relative to max
- **Loop**: Seamless with 0.1s crossfade
- **Flicker Pattern**: Frequent (5-10 glitches per 3s loop)
- **Character**: Reality is breaking down, audible strain

**Recommended Generation**:
```
Audacity/Reaper:
1. Generate 320 Hz square wave (3s)
2. Apply vibrato (0.5 Hz, ±15 Hz modulation)
3. Add white noise (30% mix, -20 dB)
4. Add distortion (soft clip, 30% drive)
5. Insert 10-15 static bursts (0.05s each, random)
6. Fade in/out 0.1s at edges
7. Export as OGG Vorbis (Q6, 128 kbps)
```

**Reference**: Think "damaged electrical transformer" - harsh, unstable, threatening.

---

## File 4: anchor_hum_critical.ogg

**Path**: `assets/audio/anchor/anchor_hum_critical.ogg`  
**Duration**: 2-3 seconds (loop seamlessly)  
**Format**: OGG Vorbis, 44.1 kHz, mono, 128 kbps

**Audio Design**:
- **Frequency**: 200-250 Hz (very distorted, almost noise)
- **Timbre**: Harsh static, reality tearing, grating
- **Volume**: 80% relative to max
- **Loop**: Seamless with 0.1s crossfade
- **Flicker Pattern**: Constant chaos (30-50 glitches per 2s loop)
- **Character**: Catastrophic failure imminent, painful to hear

**Recommended Generation**:
```
Audacity/Reaper:
1. Generate 220 Hz sawtooth wave (2s)
2. Apply heavy distortion (hard clip, 70% drive)
3. Add filtered white noise (50% mix, -10 dB, band-pass 200-500 Hz)
4. Add ring modulation (50 Hz, 40% mix)
5. Insert continuous static (entire duration, layered)
6. Fade in/out 0.1s at edges
7. Export as OGG Vorbis (Q6, 128 kbps)
```

**Reference**: Think "arc welder in a failing power grid" - aggressive, alarming, dangerous.

---

## File 5: anchor_repair.ogg

**Path**: `assets/audio/anchor/anchor_repair.ogg`  
**Duration**: **Exactly 5.0 seconds** (no loop)  
**Format**: OGG Vorbis, 44.1 kHz, stereo, 192 kbps

**Audio Design**:
- **0.0-1.0s**: Build-up (bass rumble, 60-120 Hz, crescendo)
- **1.0-2.0s**: Peak (bright chime, 880 Hz + 1320 Hz, major third harmony)
- **2.0-4.0s**: Restoration (sustained chord, C major triad, 261/330/392 Hz)
- **4.0-5.0s**: Fade out (gentle decay, reverb tail)
- **Volume**: 70% relative to max (overlays hum, doesn't drown it out)
- **Character**: Triumphant, uplifting, hope

**Recommended Generation**:
```
Audacity/Reaper:
1. Layer 1 (0.0-1.0s): Bass rumble (60 Hz sine, -10 dB, crescendo 0 → 100%)
2. Layer 2 (1.0-2.0s): Chime (880 Hz sine + 1320 Hz sine, 0 dB, attack 0.01s, decay 1.0s)
3. Layer 3 (2.0-4.0s): C major chord (261/330/392 Hz, -6 dB each, sustain)
4. Layer 4 (entire): Reverb (hall preset, 2.5s decay, 20% wet mix)
5. Add subtle "whoosh" noise (pink noise, -30 dB, 0.5-2.5s)
6. Fade out (4.0-5.0s, exponential curve)
7. Export as OGG Vorbis (Q7, 192 kbps)
```

**Reference**: Think "magical restoration spell in a fantasy game" - satisfying, rewarding, complete.

---

## File 6: echo_pickup.ogg

**Path**: `assets/audio/anchor/echo_pickup.ogg`  
**Duration**: **0.5 seconds** (no loop)  
**Format**: OGG Vorbis, 44.1 kHz, stereo, 128 kbps

**Audio Design**:
- **0.0-0.1s**: Attack (bright chime, 1760 Hz, A6 note)
- **0.1-0.3s**: Sustain (decay to 880 Hz, A5 note, octave down)
- **0.3-0.5s**: Release (fade to silence, reverb tail)
- **Volume**: 50% relative to max (subtle, non-intrusive)
- **Character**: Reward feedback, satisfying, quick

**Recommended Generation**:
```
Audacity/Reaper:
1. Generate 1760 Hz sine wave (0.1s)
2. Pitch shift down 1 octave over 0.2s (1760 → 880 Hz)
3. Apply ADSR envelope (Attack 0.01s, Decay 0.2s, Sustain 0, Release 0.2s)
4. Add subtle reverb (small room, 0.8s decay, 15% wet)
5. Add high-pass filter (300 Hz, gentle slope)
6. Export as OGG Vorbis (Q6, 128 kbps)
```

**Reference**: Think "coin pickup in Mario" - instantly recognizable, pleasant, non-distracting.

---

## Audio Implementation Checklist

### Pre-Production
- [ ] Confirm audio middleware (rodio, kira, bevy_audio?)
- [ ] Test OGG Vorbis compatibility with engine
- [ ] Set up audio asset directory structure (`assets/audio/anchor/`)

### Production
- [ ] Generate anchor_hum_stable.ogg (5-10s loop, 128 kbps)
- [ ] Generate anchor_hum_unstable.ogg (3-5s loop, 128 kbps)
- [ ] Generate anchor_hum_critical.ogg (2-3s loop, 128 kbps)
- [ ] Generate anchor_repair.ogg (5.0s one-shot, 192 kbps)
- [ ] Generate echo_pickup.ogg (0.5s one-shot, 128 kbps)
- [ ] (Optional) Generate anchor_hum_perfect.ogg (future use)

### Integration Testing
- [ ] Test seamless looping (no clicks at loop points)
- [ ] Test 3D spatial audio (position-based volume/pan)
- [ ] Test crossfade transitions (Stable → Unstable → Critical)
- [ ] Test repair sound overlay (doesn't drown out hum)
- [ ] Test Echo pickup chime (audible but non-intrusive)
- [ ] Test volume levels (20% stable, 50% unstable, 80% critical)
- [ ] Test fade in/out (1s Perfect transitions, 0.5s other transitions)

### Performance Validation
- [ ] Measure CPU usage (should be <0.1% for 10 anchors)
- [ ] Measure memory usage (OGG compressed, <2 MB total for all files)
- [ ] Test with 20 anchors (max expected in one scene)
- [ ] Verify no audio artifacts (stuttering, popping, crackling beyond design)

---

## Audio Middleware Integration

**Recommended**: Use `rodio` for simplicity, or `kira` for advanced features (crossfading, effects).

### Rodio Example (Basic)
```rust
use rodio::{Sink, Decoder, OutputStream};
use std::fs::File;
use std::io::BufReader;

fn play_anchor_hum(file_path: &str, volume: f32) -> Sink {
    let (_stream, stream_handle) = OutputStream::try_default().unwrap();
    let sink = Sink::try_new(&stream_handle).unwrap();
    
    let file = File::open(file_path).unwrap();
    let source = Decoder::new(BufReader::new(file)).unwrap();
    
    sink.set_volume(volume);
    sink.append(source);
    sink.play();
    sink
}
```

### Kira Example (Advanced Crossfading)
```rust
use kira::manager::{AudioManager, AudioManagerSettings};
use kira::sound::static_sound::{StaticSoundData, StaticSoundSettings};
use kira::tween::Tween;

fn crossfade_anchor_hum(
    manager: &mut AudioManager,
    old_sound: SoundHandle,
    new_file: &str,
    fade_duration: f64,
) {
    // Fade out old sound
    old_sound.stop(Tween::linear(fade_duration)).unwrap();
    
    // Load and play new sound with fade in
    let new_sound_data = StaticSoundData::from_file(
        new_file,
        StaticSoundSettings::new().volume(0.0).output_destination(mixer),
    ).unwrap();
    
    let mut new_sound = manager.play(new_sound_data).unwrap();
    new_sound.set_volume(0.5, Tween::linear(fade_duration)).unwrap();
}
```

---

## File Size Estimates

| File | Duration | Format | Estimated Size |
|------|----------|--------|----------------|
| anchor_hum_stable.ogg | 5-10s | OGG Q6 128 kbps | 80-160 KB |
| anchor_hum_unstable.ogg | 3-5s | OGG Q6 128 kbps | 48-80 KB |
| anchor_hum_critical.ogg | 2-3s | OGG Q6 128 kbps | 32-48 KB |
| anchor_repair.ogg | 5.0s | OGG Q7 192 kbps | 120 KB |
| echo_pickup.ogg | 0.5s | OGG Q6 128 kbps | 8 KB |
| anchor_hum_perfect.ogg | (optional) | OGG Q6 128 kbps | (future) |
| **TOTAL** | | | **~300-450 KB** |

**Performance**: All 6 files combined fit in <0.5 MB, negligible memory footprint.

---

## Troubleshooting

### Issue: Clicking at loop points
**Solution**: Add 0.1s crossfade at loop points in Audacity (Fade In 0.1s at start, Fade Out 0.1s at end).

### Issue: Hum is too loud/quiet
**Solution**: Adjust volume in audio system code (`AnchorAudioState::volume_for_state()`), not audio files.

### Issue: Repair sound overlaps awkwardly with hum
**Solution**: Reduce repair sound volume to 60-70% and add slight delay (0.1s) before repair sound starts.

### Issue: Echo pickup chime is annoying after 100 pickups
**Solution**: Add random pitch variation (±2 semitones) in code, or reduce volume to 30-40%.

---

## Next Steps

1. ✅ **Specifications complete** (this document)
2. ⏳ **Production** (generate 6 audio files using Audacity/Reaper)
3. ⏳ **Integration** (load files in `AnchorAudioSystem`, test playback)
4. ⏳ **Testing** (validate looping, crossfades, volume levels)
5. ⏳ **Polish** (adjust volumes, fade durations, pitch based on playtesting)

**Estimated Time**: 2-3 hours for audio production (if using pre-made sounds, <1 hour).

---

**End of Specifications**
