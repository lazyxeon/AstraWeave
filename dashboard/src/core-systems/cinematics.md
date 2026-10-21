# Cinematics System

> **⚠️ This chapter is outdated and pending a full rewrite (Unified Camera campaign C.7.F / post-campaign cleanup).**
>
> This chapter predates the C.7 cinematics consolidation and documents an
> outdated camera model. Specifically, it describes `CameraKey` with a
> rotation-based orientation (`rot` / Euler angles) and field/syntax that
> do not match the current `astraweave-cinematics` API. The canonical
> `CameraKey` uses a **look-at target** model:
> `CameraKey { t: Time, pos: (f32, f32, f32), look_at: (f32, f32, f32), fov_deg: f32 }`.
>
> Several APIs referenced below (track-construction helpers, camera setter
> methods) may also not exist in the current crate.
>
> **For accurate, current documentation of the cinematics camera, see:**
> - [The Camera System section in the rendering chapter](./rendering.md)
> - `docs/current/CAMERA_CONVENTIONS.md` (canonical camera conventions)
>
> A full rewrite of this chapter — re-authoring its worked examples against
> the canonical look-at model and verifying every documented API symbol
> against the actual crate surface — is tracked as C.7.F in the
> post-campaign cleanup queue. Until then, treat this chapter's specifics
> as unreliable.

---

> **Status**: Production Ready  
> **Crate**: `astraweave-cinematics`  
> **Coverage**: ~85%

The cinematics system provides a timeline-based sequencer for creating cutscenes, scripted events, and cinematic sequences with synchronized camera, animation, audio, and visual effects tracks.

## Core Types

| Type | Description |
|------|-------------|
| `Time` | Precise time representation (seconds/milliseconds) |
| `Track` | Track variant (Camera, Animation, Audio, Fx) |
| `CameraKey` | Camera keyframe with position, rotation, FOV |
| `Timeline` | Collection of tracks with duration |
| `Sequencer` | Playback engine with seek and step |
| `SequencerEvent` | Events emitted during playback |

---

## Quick Start

```rust
use astraweave_cinematics::{Time, Timeline, Track, CameraKey, Sequencer};

// Create a 10-second timeline
let mut timeline = Timeline::new("intro_cutscene", 10.0);

// Add camera keyframes
let keyframes = vec![
    CameraKey::new(Time::zero(), [0.0, 5.0, -10.0], [0.0, 0.0, 0.0], 60.0),
    CameraKey::new(Time::from_secs(5.0), [10.0, 8.0, -5.0], [0.0, 45.0, 0.0], 55.0),
    CameraKey::new(Time::from_secs(10.0), [0.0, 3.0, 0.0], [0.0, 90.0, 0.0], 70.0),
];
timeline.add_camera_track(keyframes);

// Add audio
timeline.add_audio_track("music/intro.ogg", Time::zero(), 0.8);

// Add animation trigger
timeline.add_track(Track::animation(1, "character_wave", Time::from_secs(2.0)));

// Add FX
timeline.add_track(Track::fx("explosion", Time::from_secs(8.0), serde_json::json!({
    "scale": 2.0,
    "particles": 500
})));

// Playback
let mut sequencer = Sequencer::new();
let dt = 1.0 / 60.0; // 60 FPS

loop {
    match sequencer.step(dt, &timeline) {
        Ok(events) => {
            for event in events {
                handle_event(event);
            }
        }
        Err(_) => break, // End of timeline
    }
}
```

---

## Architecture

### Timeline Structure

```
Timeline "intro_cutscene" (10.0s)
├── Track::Camera
│   ├── Keyframe @ 0.0s (pos, rot, fov)
│   ├── Keyframe @ 5.0s
│   └── Keyframe @ 10.0s
├── Track::Audio "music/intro.ogg" @ 0.0s
├── Track::Animation target=1 "character_wave" @ 2.0s
└── Track::Fx "explosion" @ 8.0s
```

### Playback Flow

```
┌───────────────────────────────────────────────────┐
│                    Sequencer                       │
│  ┌─────────┐    ┌───────────┐    ┌─────────────┐  │
│  │  seek() │ →  │  step(dt) │ →  │   events    │  │
│  └─────────┘    └───────────┘    └─────────────┘  │
│                        │                           │
│                        ▼                           │
│  ┌─────────────────────────────────────────────┐  │
│  │ For each track, emit events where           │  │
│  │ start_time > previous_t && start_time <= t  │  │
│  └─────────────────────────────────────────────┘  │
└───────────────────────────────────────────────────┘
```

---

## Time API

The `Time` type provides precision time handling:

```rust
use astraweave_cinematics::Time;

// Construction
let t1 = Time::zero();                  // 0.0s
let t2 = Time::from_secs(5.0);          // 5.0s
let t3 = Time::from_millis(1500.0);     // 1.5s

// Access
println!("Seconds: {}", t2.as_secs());   // 5.0
println!("Millis: {}", t2.as_millis());  // 5000.0

// Arithmetic
let sum = t2 + t3;                       // 6.5s
let diff = t2 - t3;                      // 3.5s
let added = t2.add_secs(2.5);            // 7.5s

// Utilities
let clamped = t2.clamp(Time::zero(), Time::from_secs(4.0)); // 4.0s
let lerped = Time::zero().lerp(Time::from_secs(10.0), 0.3); // 3.0s

// Display
println!("{}", Time::from_secs(2.5));    // "2.50s"
println!("{}", Time::from_millis(500.0)); // "500ms"
```

---

## Track Types

### Camera Track

Controls camera position, rotation, and field of view:

```rust
use astraweave_cinematics::{Track, CameraKey, Time};

let track = Track::camera(vec![
    CameraKey {
        t: Time::zero(),
        pos: [0.0, 5.0, -10.0],
        rot: [0.0, 0.0, 0.0],      // Euler angles (degrees)
        fov: 60.0,                  // Field of view
    },
    CameraKey {
        t: Time::from_secs(5.0),
        pos: [10.0, 8.0, -5.0],
        rot: [0.0, 45.0, 0.0],
        fov: 55.0,
    },
]);

assert!(track.is_camera());
assert_eq!(track.keyframe_count(), Some(2));
```

### Animation Track

Triggers character/object animations:

```rust
let track = Track::animation(
    42,                        // Target entity ID
    "run_forward",             // Animation clip name
    Time::from_secs(1.5),      // Start time
);

assert!(track.is_animation());
assert_eq!(track.start_time(), Some(Time::from_secs(1.5)));
```

### Audio Track

Plays sound effects or music:

```rust
let track = Track::audio(
    "sfx/explosion.wav",       // Audio clip path
    Time::from_secs(3.0),      // Start time
    0.8,                       // Volume (0.0 - 1.0)
);

assert!(track.is_audio());
```

### FX Track

Triggers visual effects with parameters:

```rust
use serde_json::json;

let track = Track::fx(
    "particle_burst",          // Effect name
    Time::from_secs(2.0),      // Start time
    json!({
        "count": 100,
        "color": [1.0, 0.5, 0.0],
        "lifetime": 2.0
    }),
);

assert!(track.is_fx());
```

---

## Timeline API

### Creating Timelines

```rust
use astraweave_cinematics::Timeline;

// Named timeline with duration
let mut timeline = Timeline::new("boss_intro", 30.0);

// Empty timeline
let empty = Timeline::empty();
assert!(empty.is_empty());
```

### Building Timelines

```rust
let mut timeline = Timeline::new("action_sequence", 15.0);

// Add tracks
timeline.add_camera_track(camera_keyframes);
timeline.add_audio_track("music/action.ogg", Time::zero(), 1.0);
timeline.add_track(Track::animation(1, "attack", Time::from_secs(5.0)));

// Track counts
println!("Camera tracks: {}", timeline.camera_track_count());
println!("Audio tracks: {}", timeline.audio_track_count());
println!("Animation tracks: {}", timeline.animation_track_count());
println!("FX tracks: {}", timeline.fx_track_count());
println!("Total tracks: {}", timeline.track_count());
println!("Total keyframes: {}", timeline.total_keyframes());
```

### Timeline Properties

```rust
let timeline = Timeline::new("example", 10.0);

println!("Name: {}", timeline.name);
println!("Duration: {}s", timeline.duration_secs());
println!("Empty: {}", timeline.is_empty());

// Display
println!("{}", timeline); // Timeline("example", duration=10.00s, 0 tracks)
```

---

## Sequencer API

### Basic Playback

```rust
use astraweave_cinematics::Sequencer;

let mut seq = Sequencer::new();
let dt = 1.0 / 60.0;  // 60 FPS

// Step through timeline
loop {
    match seq.step(dt, &timeline) {
        Ok(events) => {
            for event in events {
                process_event(event);
            }
        }
        Err(SeqError::Range(_)) => {
            println!("Timeline complete");
            break;
        }
    }
}
```

### Seeking

```rust
let mut seq = Sequencer::new();

// Jump to specific time
seq.seek(Time::from_secs(5.0));

// Current time
println!("Current: {}", seq.t);
```

### Event Handling

```rust
use astraweave_cinematics::SequencerEvent;

fn handle_event(event: SequencerEvent) {
    match event {
        SequencerEvent::CameraKey(key) => {
            camera.set_position(key.pos);
            camera.set_rotation(key.rot);
            camera.set_fov(key.fov);
        }
        SequencerEvent::AnimStart { target, clip } => {
            if let Some(entity) = world.get_entity(target) {
                animator.play(entity, &clip);
            }
        }
        SequencerEvent::AudioPlay { clip, volume } => {
            audio.play(&clip, volume);
        }
        SequencerEvent::FxTrigger { name, params } => {
            fx_system.trigger(&name, params);
        }
    }
}
```

---

## Serialization

Timelines are fully serializable with Serde:

```rust
use astraweave_cinematics::Timeline;

// Save to JSON
let json = serde_json::to_string_pretty(&timeline)?;
std::fs::write("cutscene.json", &json)?;

// Load from JSON
let loaded: Timeline = serde_json::from_str(&json)?;

// Save to RON (common in game dev)
let ron = ron::to_string(&timeline)?;
```

**Example JSON**:
```json
{
  "name": "intro",
  "duration": 10.0,
  "tracks": [
    {
      "Camera": {
        "keyframes": [
          { "t": 0.0, "pos": [0, 5, -10], "rot": [0, 0, 0], "fov": 60 },
          { "t": 5.0, "pos": [10, 8, -5], "rot": [0, 45, 0], "fov": 55 }
        ]
      }
    },
    {
      "Audio": {
        "clip": "music/intro.ogg",
        "start": 0.0,
        "volume": 0.8
      }
    }
  ]
}
```

---

## Integration Examples

### With ECS

```rust
use astraweave_ecs::World;
use astraweave_cinematics::{Sequencer, Timeline};

fn cinematics_system(world: &mut World, timeline: &Timeline, seq: &mut Sequencer, dt: f32) {
    if let Ok(events) = seq.step(dt, timeline) {
        for event in events {
            match event {
                SequencerEvent::CameraKey(key) => {
                    // Update camera component
                    if let Some(camera) = world.query_mut::<CameraComponent>().next() {
                        camera.position = key.pos.into();
                        camera.rotation = key.rot.into();
                        camera.fov = key.fov;
                    }
                }
                SequencerEvent::AnimStart { target, clip } => {
                    // Trigger animation on entity
                    if let Some(anim) = world.get_component_mut::<Animator>(target) {
                        anim.play(&clip);
                    }
                }
                // ... handle other events
            }
        }
    }
}
```

### With Game State

```rust
enum GameState {
    Playing,
    Cutscene { timeline: Timeline, sequencer: Sequencer },
}

impl GameState {
    fn update(&mut self, dt: f32) -> Option<SequencerEvent> {
        match self {
            GameState::Cutscene { timeline, sequencer } => {
                match sequencer.step(dt, timeline) {
                    Ok(events) => {
                        for event in events {
                            // Handle events
                        }
                        None
                    }
                    Err(_) => {
                        // Cutscene complete, return to gameplay
                        *self = GameState::Playing;
                        None
                    }
                }
            }
            _ => None,
        }
    }
}
```

---

## Performance

| Operation | Latency | Notes |
|-----------|---------|-------|
| `Time` creation | ~1 ns | Zero-cost abstraction |
| `step()` per track | ~50 ns | Linear scan of events |
| Event emission | ~100 ns | Allocation per event |
| Full timeline step | ~500 ns | Typical 10-track timeline |

### Optimization Tips

1. **Pre-sort tracks by start time** for faster event lookup
2. **Reuse sequencers** instead of creating new ones
3. **Batch event handling** when multiple events fire simultaneously
4. **Use `seek()` sparingly** - stepping is more efficient

---

## Common Patterns

### Looping Cutscenes

```rust
let mut seq = Sequencer::new();

loop {
    match seq.step(dt, &timeline) {
        Ok(events) => handle_events(events),
        Err(_) => {
            // Loop back to start
            seq.seek(Time::zero());
        }
    }
}
```

### Skippable Cutscenes

```rust
fn update_cutscene(seq: &mut Sequencer, timeline: &Timeline, dt: f32, skip_pressed: bool) {
    if skip_pressed {
        // Jump to end
        seq.seek(timeline.duration);
        return;
    }
    
    if let Ok(events) = seq.step(dt, timeline) {
        handle_events(events);
    }
}
```

### Branching Cutscenes

```rust
struct BranchingCutscene {
    timelines: HashMap<String, Timeline>,
    current: String,
    sequencer: Sequencer,
}

impl BranchingCutscene {
    fn switch_to(&mut self, name: &str) {
        self.current = name.to_string();
        self.sequencer.seek(Time::zero());
    }
    
    fn update(&mut self, dt: f32) -> Vec<SequencerEvent> {
        let timeline = &self.timelines[&self.current];
        self.sequencer.step(dt, timeline).unwrap_or_default()
    }
}
```

---

## See Also

- [API Reference: Cinematics](../api/cinematics.md)
- [Audio System](./audio.md) - Audio integration
- [Animation System](./animation.md) - Character animation
- [Camera System](./camera.md) - Camera controls
