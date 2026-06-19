# Phase 8.4: Production Audio System Implementation Plan

**Document Version**: 1.0  
**Date**: October 14, 2025  
**Duration**: 2-3 weeks  
**Dependencies**: Phase 8.1 (UI for mixer controls)

---

## Executive Summary

**Mission**: Transform AstraWeave's audio from "functional prototype" to "production-quality sound system" with mixer, dynamic music, occlusion, and editor tools.

**Current State** (From Roadmap Review):
- ✅ **Spatial Audio**: Complete (`AudioEngine`, `SpatialSink`, listener pose)
- ✅ **Dialogue System**: Complete (`DialoguePlayer`, `VoiceBank`, TTS adapter)
- ✅ **Music Crossfading**: Complete (`MusicChannel` with dual-sink A/B system)
- ✅ **Basic Buses**: Exists (master_volume, music_base_volume, voice_base_volume, sfx_base_volume)
- ⚠️ **Audio Mixer**: Bus structure exists but needs UI integration
- ❌ **Dynamic Music Layers**: Single-track crossfade only (no layering)
- ❌ **Audio Occlusion**: Not implemented (sounds not blocked by walls)
- ❌ **Reverb Zones**: Not implemented (no environmental acoustics)
- ❌ **In-Editor Audio Tools**: TOML-based workflow only

**Target State** (Phase 8 Complete):
- ✅ Full audio mixer with 4+ buses (master, music, SFX, voice, ambient)
- ✅ Per-bus controls (volume, mute, solo, EQ, compression)
- ✅ Dynamic music layers (intro, loop, combat, tension, ending)
- ✅ Audio occlusion (raycast-based, sounds blocked by geometry)
- ✅ Reverb zones (trigger volumes for environmental acoustics)
- ✅ In-editor audio preview (play sounds, test spatial audio)
- ✅ Audio mixer UI panel (Phase 8.1 integration)

**Timeline**: 2-3 weeks (10-15 working days)

**Success Criteria**: Veilweaver Demo Level has AAA-quality audio matching modern indie games

---

## Week 1: Audio Mixer & Bus System

**Goal**: Expose existing (in-design) bus infrastructure as a mixer

### Day 1-2: Audio Bus Architecture

**Tasks**:
1. **Refactor Existing Bus System**:
   - File: `astraweave-audio/src/engine.rs`
   - Current: `master_volume`, `music_base_volume`, `voice_base_volume`, `sfx_base_volume` (separate fields)
   - New: `HashMap<BusId, AudioBus>` (extensible, supports custom buses)
   - Bus struct:
     ```rust
     pub struct AudioBus {
         pub id: BusId,
         pub name: String,
         pub volume: f32,           // 0.0 - 1.0
         pub muted: bool,
         pub soloed: bool,          // Only this bus audible (debugging)
         pub parent: Option<BusId>, // For bus hierarchy (SFX → UI_SFX, GAME_SFX)
         pub effects: Vec<AudioEffect>, // EQ, compression, reverb
     }
     
     pub enum BusId {
         Master,
         Music,
         SFX,
         Voice,
         Ambient,
         Custom(String),
     }
     ```
   - Validate: Existing audio engine works with new bus system

2. **Bus Hierarchy**:
   - **Structure**:
     ```
     Master (100%)
       ├── Music (80%)
       ├── SFX (90%)
       │   ├── UI_SFX (100%)
       │   └── GAME_SFX (100%)
       ├── Voice (100%)
       └── Ambient (60%)
     ```
   - **Mixing**: Child volume multiplied by parent (e.g., UI_SFX = 0.9 × 1.0 = 0.9)
   - **Mute propagation**: Muting parent mutes all children
   - Validate: Hierarchy works correctly (mute, volume, solo)

3. **Bus Assignment API**:
   - API: `engine.play_on_bus(sound, BusId::SFX)`
   - Default: SFX bus if not specified
   - Validation: Sounds route to correct bus
   - Example:
     ```rust
     engine.play_on_bus("footstep.ogg", BusId::SFX);
     engine.play_on_bus("music.ogg", BusId::Music);
     engine.play_on_bus("dialogue.ogg", BusId::Voice);
     ```

**Deliverables**:
- `AudioBus` struct with hierarchy support
- `HashMap<BusId, AudioBus>` in AudioEngine
- Bus assignment API

**Success Criteria**:
- ✅ Bus hierarchy works (parent/child volume multiplication)
- ✅ Mute/solo work correctly (with propagation)
- ✅ Sounds route to correct buses

---

### Day 3-4: Mixer UI Integration (Phase 8.1 Dependency)

**Tasks**:
1. **Audio Mixer Panel** (egui in `aw_editor`):
   - UI: Vertical faders for each bus (Master, Music, SFX, Voice, Ambient)
   - Controls: Volume slider (0-100%), mute button, solo button
   - Meters: Real-time volume level visualization (VU meter)
   - Validate: Faders control audio in real-time

2. **In-Game Mixer UI** (Phase 8.1 integration):
   - Settings menu: Audio tab with simple volume sliders
   - Controls: Master, Music, SFX, Voice volumes
   - Persist: Save to PlayerProfile (Phase 8.3 integration)
   - Validate: Settings persist across sessions

3. **Mixer Snapshot System**:
   - **Use case**: "Combat" snapshot lowers music, raises SFX
   - **API**: `mixer.apply_snapshot("combat")` → Interpolate to new volumes
   - **Snapshots**: Define in TOML (e.g., `combat.toml`, `exploration.toml`)
   - Example:
     ```toml
     [combat]
     master = 1.0
     music = 0.4   # Lower music during combat
     sfx = 1.2     # Boost combat sounds
     voice = 1.0
     ambient = 0.6
     ```
   - Validate: Snapshots blend smoothly (no popping)

**Deliverables**:
- Audio mixer panel in editor
- In-game audio settings UI
- Mixer snapshot system

**Success Criteria**:
- ✅ Real-time mixer control in editor
- ✅ Player can adjust volumes in-game
- ✅ Mixer snapshots blend smoothly (no popping)

---

### Day 5: Audio Effects & Processing

**Tasks**:
1. **EQ (Equalizer)** (Optional, low priority):
   - **Use case**: Low-pass filter for underwater effect
   - **Implementation**: Use `rodio` filters or DSP library
   - **Defer if complex**: Can add in Phase 9

2. **Compression** (Optional, low priority):
   - **Use case**: Ducking (lower music when dialogue plays)
   - **Current**: Music ducking already implemented (MusicChannel)
   - **Skip**: Existing ducking is sufficient

3. **Validation & Testing**:
   - Test: Adjust all bus volumes, verify correct mixing
   - Test: Mute/solo buses, verify isolation
   - Test: Apply mixer snapshot, verify smooth blend
   - Performance: <0.1ms overhead for mixer (negligible)

**Deliverables**:
- Validated mixer system
- Optional: EQ/compression (defer if time-constrained)

**Success Criteria**:
- ✅ Mixer works correctly (volume, mute, solo)
- ✅ No audio artifacts (popping, clipping)
- ✅ Performance: <0.1ms overhead

---

## Week 2: Dynamic Music & Adaptive Audio

**Goal**: Extend MusicChannel to support multi-layer dynamic music

### Day 6-7: Music Layer System

**Tasks**:
1. **Layer Architecture**:
   - **Current**: MusicChannel with 2 sinks (A/B crossfade)
   - **New**: Support 4+ simultaneous layers (intro, loop, combat, tension)
   - **Example**:
     ```
     Base Loop (always playing, 60% volume)
     + Combat Layer (when enemies nearby, 0-100% volume)
     + Tension Layer (when low health, 0-100% volume)
     + Ending Layer (when boss defeated, fade in)
     ```
   - **Implementation**: `Vec<MusicLayer>` with per-layer volume control
   - Validate: Multiple layers play simultaneously

2. **Layer Blending**:
   - **Crossfade**: Smooth volume transitions (1-2 second lerp)
   - **Sync**: All layers start at same timestamp (beat-synchronized)
   - **Looping**: Layers loop seamlessly (no gap)
   - Validate: Layers blend smoothly, stay in sync

3. **Layer Control API**:
   ```rust
   music.set_layer_volume("combat", 0.8);   // Fade in combat layer
   music.set_layer_volume("tension", 0.0);  // Fade out tension layer
   music.crossfade_to("boss_fight", 2.0);   // Crossfade to new track
   ```
   - Validation: API works in real-time (no stuttering)

**Deliverables**:
- Multi-layer music system
- Layer blending with sync
- Layer control API

**Success Criteria**:
- ✅ 4+ layers play simultaneously without artifacts
- ✅ Layers stay synchronized (beat-aligned)
- ✅ Smooth volume transitions (no popping)

---

### Day 8-9: Adaptive Music System

**Tasks**:
1. **Music State Machine**:
   - **States**: Exploration, Combat, Stealth, Boss Fight, Victory
   - **Transitions**: Define triggers (e.g., enemy nearby → Combat)
   - **Layers**: Each state has different layer mix
   - Example:
     ```rust
     match game_state {
         GameState::Exploration => {
             music.set_layer_volume("combat", 0.0);
             music.set_layer_volume("base", 1.0);
         }
         GameState::Combat => {
             music.set_layer_volume("combat", 1.0);
             music.set_layer_volume("base", 0.6);
         }
     }
     ```
   - Validate: Music adapts to gameplay

2. **Music Cues**:
   - **Stingers**: Short musical hits (e.g., "enemy spotted" sound)
   - **Triggers**: Gameplay events (enemy spawn, low health, achievement)
   - **API**: `music.play_stinger("enemy_spotted.ogg")`
   - Validate: Stingers play at correct time, don't interrupt music

3. **Vertical & Horizontal Remixing**:
   - **Vertical**: Layer on/off (e.g., add drums in combat)
   - **Horizontal**: Crossfade between tracks (e.g., exploration → combat)
   - **Combo**: Use both (layer intensity + track switch)
   - Validate: Music system supports both techniques

**Deliverables**:
- Music state machine
- Stinger system
- Vertical + horizontal remixing

**Success Criteria**:
- ✅ Music adapts to gameplay (exploration → combat → victory)
- ✅ Stingers play at correct moments
- ✅ No jarring transitions (smooth blending)

---

### Day 10: Music Integration & Testing

**Tasks**:
1. **Integrate with `unified_showcase`**:
   - Music: Base loop with combat layer
   - Trigger: Combat layer fades in when near enemies
   - Validate: Music responds to gameplay

2. **Integrate with Veilweaver Demo**:
   - Music: Exploration theme with combat/boss variants
   - Layers: Base, combat, tension, victory
   - Validate: Music enhances gameplay, not distracting

3. **Performance Validation**:
   - Measure: CPU usage for 4+ simultaneous music layers
   - Target: <1% CPU overhead (rodio is efficient)
   - Validate: No audio dropouts or stuttering

**Deliverables**:
- Music integration in examples
- Performance validation

**Success Criteria**:
- ✅ Music system works in real game scenarios
- ✅ Performance: <1% CPU for 4 layers
- ✅ No audio artifacts

---

## Week 3: Audio Occlusion & Reverb Zones

**Goal**: Add environmental audio effects for realism

### Day 11-12: Audio Occlusion (Raycast-Based)

**Tasks**:
1. **Occlusion Algorithm**:
   - **Technique**: Raycast from listener to sound emitter
   - **Check**: Does ray hit geometry? (wall, floor, etc.)
   - **Attenuation**: If occluded, reduce volume by 50-90% (configurable)
   - **Low-pass filter**: Optional (muffled sound through walls)
   - Example:
     ```rust
     let ray = Ray { origin: listener_pos, dir: (emitter_pos - listener_pos).normalize() };
     if physics.raycast(ray, max_distance).is_some() {
         volume *= occlusion_factor; // 0.1 - 0.5
     }
     ```
   - Validate: Sounds muffled when behind walls

2. **Performance Optimization**:
   - **Problem**: Raycast per emitter per frame is expensive
   - **Mitigation 1**: Only raycast for nearby emitters (<50m)
   - **Mitigation 2**: Update occlusion every 5-10 frames (not every frame)
   - **Mitigation 3**: Spatial hash to cull distant emitters
   - Target: <0.5ms for 100 emitters
   - Validate: Performance acceptable

3. **Occlusion Smoothing**:
   - **Problem**: Instant occlusion causes popping
   - **Solution**: Lerp volume over 0.1-0.2 seconds
   - **Implementation**: `current_volume = lerp(current, target, dt / smoothing_time)`
   - Validate: Smooth occlusion transitions

**Deliverables**:
- Raycast-based occlusion system
- Performance optimization (spatial culling, update throttling)
- Occlusion smoothing

**Success Criteria**:
- ✅ Sounds muffled when behind walls (realistic occlusion)
- ✅ Performance: <0.5ms for 100 emitters
- ✅ Smooth transitions (no popping)

---

### Day 13-14: Reverb Zones

**Tasks**:
1. **Reverb Zone System**:
   - **Zone**: Trigger volume (box or sphere) in world
   - **Parameters**: Reverb type (cave, hall, room, sewer, canyon)
   - **Effect**: Apply reverb to sounds inside zone
   - Example:
     ```rust
     pub struct ReverbZone {
         pub bounds: AABB,          // Axis-aligned bounding box
         pub reverb_type: ReverbType, // Cave, Hall, Room, etc.
         pub wet_mix: f32,          // 0.0 - 1.0 (reverb intensity)
     }
     
     pub enum ReverbType {
         Cave,     // Long reverb tail, dark tone
         Hall,     // Medium reverb, bright tone
         Room,     // Short reverb, neutral tone
         Sewer,    // Metallic, harsh reverb
         Canyon,   // Very long reverb, echo
     }
     ```
   - Validate: Reverb applies when entering zone

2. **Reverb Implementation**:
   - **Option 1**: Use `rodio` reverb filter (if available)
   - **Option 2**: Use external DSP library (e.g., `reverb-rs`, if exists)
   - **Option 3**: Defer to Phase 9 if too complex (use pre-baked reverb)
   - **Fallback**: Simple echo effect (delay + feedback)
   - Validate: Reverb sounds realistic

3. **Zone Blending**:
   - **Problem**: Transitioning between zones causes sudden reverb change
   - **Solution**: Blend reverb when near zone boundary
   - **Implementation**: Interpolate wet_mix based on distance to zone center
   - Validate: Smooth reverb transitions

**Deliverables**:
- Reverb zone system (trigger volumes)
- Reverb implementation (rodio filter or simple echo)
- Zone blending for smooth transitions

**Success Criteria**:
- ✅ Reverb zones enhance environmental acoustics
- ✅ 5+ reverb types (cave, hall, room, sewer, canyon)
- ✅ Smooth transitions between zones

---

### Day 15: In-Editor Audio Tools

**Tasks**:
1. **Audio Preview Panel** (in `aw_editor`):
   - UI: List of audio assets with play/stop buttons
   - Preview: Play sound at listener position (no spatial audio)
   - Waveform: Optional waveform visualization (defer if complex)
   - Validate: Artists can preview sounds quickly

2. **Spatial Audio Testing**:
   - UI: Place emitter in 3D viewport (drag & drop)
   - Controls: Play sound, adjust volume, change bus
   - Visualization: Show emitter position, listener position, occlusion rays
   - Validate: Artists can test spatial audio interactively

3. **Reverb Zone Editing**:
   - UI: Create reverb zone (box or sphere)
   - Parameters: Reverb type, wet mix, size
   - Preview: Hear reverb while moving in zone
   - Validate: Level designers can place reverb zones easily

**Deliverables**:
- Audio preview panel in editor
- Spatial audio testing tools
- Reverb zone editing tools

**Success Criteria**:
- ✅ Artists can preview audio assets in editor
- ✅ Spatial audio testable without running game
- ✅ Reverb zones editable in 3D viewport

---

## Integration & Testing

### Testing Strategy

**Unit Tests**:
- Bus hierarchy (parent/child volume multiplication)
- Mixer snapshots (blend correctness)
- Layer synchronization (beat-aligned looping)
- Occlusion raycast (correct attenuation)
- Reverb zone (trigger volume detection)

**Integration Tests**:
- Full audio pipeline (mixer + music + occlusion + reverb)
- `hello_companion`: Music adapts to AI combat state
- `unified_showcase`: Spatial audio + occlusion in biome scenes
- Veilweaver Demo: Full audio integration (music, SFX, dialogue, reverb)

**Performance Tests**:
- 100 simultaneous emitters with occlusion (<1ms)
- 4 music layers with real-time blending (<1% CPU)
- Mixer overhead (negligible, <0.1ms)

**Manual Tests**:
- Walk through wall → hear occlusion
- Enter cave → hear reverb
- Combat starts → music intensity increases
- Adjust mixer → hear volume changes in real-time

---

## Success Criteria (Phase 8.4 Complete)

### Functionality

- ✅ **Audio Mixer**: 4+ buses (master, music, SFX, voice, ambient) with hierarchy
- ✅ **Mixer UI**: Real-time control in editor + in-game settings menu
- ✅ **Mixer Snapshots**: Smooth blending between audio states (combat, exploration)
- ✅ **Dynamic Music**: 4+ layers with beat-synchronized blending
- ✅ **Adaptive Music**: Music responds to gameplay (exploration → combat → victory)
- ✅ **Stingers**: Musical cues for gameplay events (enemy spotted, low health)
- ✅ **Audio Occlusion**: Sounds muffled behind walls (raycast-based)
- ✅ **Reverb Zones**: Environmental acoustics (cave, hall, room, etc.)
- ✅ **Editor Tools**: Audio preview, spatial audio testing, reverb zone editing

### Performance

- ✅ **Mixer Overhead**: <0.1ms for bus mixing
- ✅ **Music Layers**: <1% CPU for 4 simultaneous layers
- ✅ **Occlusion**: <0.5ms for 100 emitters
- ✅ **Reverb**: <1ms for reverb processing (per-zone)
- ✅ **Total Audio**: <3ms budget for all audio systems

### Code Quality

- ✅ **Zero `.unwrap()`**: All audio code uses proper error handling
- ✅ **Zero panics**: Missing audio files handled gracefully
- ✅ **50%+ test coverage**: Unit + integration tests for audio systems
- ✅ **Documentation**: API docs for AudioEngine, MusicChannel, ReverbZone

### Integration

- ✅ **Phase 8.1 UI**: Mixer panel + in-game settings integrated
- ✅ **Phase 8.3 Save/Load**: Audio settings persist in PlayerProfile
- ✅ **Veilweaver Demo**: AAA-quality audio (music, SFX, dialogue, reverb)

---

## Dependencies & Risks

### Dependencies

**Upstream** (Blocks this work):
- Phase 8.1 (UI): Needed for mixer panel and in-game settings

**Downstream** (Blocked by this work):
- Veilweaver Demo Level (needs production audio for polish)

### Risks

**High Risk**:
1. **Reverb Implementation Complexity**: DSP is complex, rodio may not have built-in reverb
   - Mitigation: Use simple echo effect (delay + feedback) as fallback
   - Defer: Full reverb to Phase 9, use pre-baked reverb for demo

**Medium Risk**:
2. **Music Layer Synchronization**: Keeping layers beat-aligned is tricky
   - Mitigation: Use rodio's `Sink::try_seek()` to sync playback positions
   - Fallback: Accept slight drift (not noticeable for most players)

3. **Occlusion Performance**: Raycast per emitter can be expensive
   - Mitigation: Spatial culling + update throttling (every 5-10 frames)
   - Fallback: Limit to 50 nearby emitters with occlusion

**Low Risk**:
4. **Mixer Snapshot Blending**: Linear interpolation may cause popping
   - Mitigation: Use exponential or logarithmic blending (smoother)
   - Easy fix: Standard audio engineering technique

---

## Deliverables Checklist

### Code

- [ ] Audio bus system with hierarchy (`AudioBus`, `BusId`)
- [ ] Mixer UI panel (editor) and in-game settings menu
- [ ] Mixer snapshot system (TOML-based definitions)
- [ ] Multi-layer music system (4+ simultaneous layers)
- [ ] Adaptive music system (state machine + stingers)
- [ ] Audio occlusion (raycast-based attenuation)
- [ ] Reverb zones (trigger volumes with 5+ types)
- [ ] In-editor audio tools (preview, spatial testing, zone editing)

### Documentation

- [ ] `AUDIO_MIXER_API.md`: API docs for mixer, buses, snapshots
- [ ] `DYNAMIC_MUSIC_GUIDE.md`: How to create adaptive music
- [ ] `SPATIAL_AUDIO_GUIDE.md`: Occlusion, reverb zones, best practices
- [ ] `AUDIO_INTEGRATION.md`: How to integrate audio in gameplay code

### Examples

- [ ] `audio_mixer_demo`: Mixer UI demonstration
- [ ] `dynamic_music_demo`: Adaptive music showcase
- [ ] `spatial_audio_demo`: Occlusion + reverb demonstration (already exists, enhance it)

### Tests

- [ ] Unit tests: Bus hierarchy, mixer snapshots, layer sync, occlusion, reverb
- [ ] Integration tests: Full audio pipeline, examples integration
- [ ] Performance tests: 100 emitters, 4 music layers, mixer overhead
- [ ] Manual tests: Occlusion through walls, reverb in zones, music adaptation

---

## Timeline Summary

| Week | Days | Focus | Deliverables |
|------|------|-------|--------------|
| 1 | 1-5 | Audio Mixer & Bus System | Bus architecture, mixer UI, snapshots, effects |
| 2 | 6-10 | Dynamic Music & Adaptive Audio | Music layers, state machine, stingers, integration |
| 3 | 11-15 | Occlusion & Reverb Zones | Raycast occlusion, reverb zones, editor tools |

**Total Duration**: 2-3 weeks (10-15 days)

**Estimated Effort**: 80-120 hours (1 FTE)

---

## Next Steps

1. **Read this plan**: Understand scope, timeline, risks
2. **Create Master Integration Plan**: Coordinate all 4 Phase 8 priorities
3. **Begin Phase 8.1**: UI framework (needed for mixer panel)
4. **Begin Phase 8.4 Week 1**: Audio mixer (after Phase 8.1 Week 2 complete)

---

**Document Status**: Implementation plan ready for execution  
**Last Updated**: October 14, 2025  
**Next Document**: PHASE_8_MASTER_INTEGRATION_PLAN.md
