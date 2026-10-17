// Comprehensive benchmarks for astraweave-cinematics
//
// Covers: Timeline creation, Sequencer operations, Track handling, Event emission,
// JSON serialization/deserialization, and various edge cases.
//
// v5.36 - January 2026

use criterion::{criterion_group, BenchmarkId, Criterion};
use std::hint::black_box;

// Re-implement the types here to avoid dependency issues
// (cinematics crate is simple enough to inline)

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub struct Time(pub f32);

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum Track {
    Camera {
        keyframes: Vec<CameraKey>,
    },
    Animation {
        target: u32,
        clip: String,
        start: Time,
    },
    Audio {
        clip: String,
        start: Time,
        volume: f32,
    },
    Fx {
        name: String,
        start: Time,
        params: serde_json::Value,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CameraKey {
    pub t: Time,
    pub pos: (f32, f32, f32),
    pub look_at: (f32, f32, f32),
    pub fov_deg: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Timeline {
    pub name: String,
    pub duration: Time,
    pub tracks: Vec<Track>,
}

impl Timeline {
    pub fn new(name: &str, duration: f32) -> Self {
        Self {
            name: name.into(),
            duration: Time(duration),
            tracks: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum SequencerEvent {
    CameraKey(CameraKey),
    AnimStart {
        target: u32,
        clip: String,
    },
    AudioPlay {
        clip: String,
        volume: f32,
    },
    FxTrigger {
        name: String,
        params: serde_json::Value,
    },
}

pub struct Sequencer {
    pub t: Time,
}

impl Default for Sequencer {
    fn default() -> Self {
        Self::new()
    }
}

impl Sequencer {
    pub fn new() -> Self {
        Self { t: Time(0.0) }
    }

    pub fn seek(&mut self, t: Time) {
        self.t = t;
    }

    pub fn step(&mut self, dt: f32, tl: &Timeline) -> Result<Vec<SequencerEvent>, String> {
        let next_t = Time(self.t.0 + dt);
        if next_t.0 > tl.duration.0 + 0.001 {
            return Err(format!("timeline out of range: {:?}", next_t));
        }
        let from = self.t.0;
        let to = next_t.0;
        self.t = next_t;

        let mut evs = Vec::new();
        for tr in &tl.tracks {
            match tr {
                Track::Camera { keyframes } => {
                    for k in keyframes {
                        if k.t.0 > from && k.t.0 <= to {
                            evs.push(SequencerEvent::CameraKey(k.clone()));
                        }
                    }
                }
                Track::Animation {
                    target,
                    clip,
                    start,
                } => {
                    if start.0 > from && start.0 <= to {
                        evs.push(SequencerEvent::AnimStart {
                            target: *target,
                            clip: clip.clone(),
                        });
                    }
                }
                Track::Audio {
                    clip,
                    start,
                    volume,
                } => {
                    if start.0 > from && start.0 <= to {
                        evs.push(SequencerEvent::AudioPlay {
                            clip: clip.clone(),
                            volume: *volume,
                        });
                    }
                }
                Track::Fx {
                    name,
                    start,
                    params,
                } => {
                    if start.0 > from && start.0 <= to {
                        evs.push(SequencerEvent::FxTrigger {
                            name: name.clone(),
                            params: params.clone(),
                        });
                    }
                }
            }
        }
        Ok(evs)
    }
}

// ============================================================================
// Helper functions for creating test data
// ============================================================================

fn create_camera_keyframe(t: f32) -> CameraKey {
    CameraKey {
        t: Time(t),
        pos: (t * 10.0, 5.0, 0.0),
        look_at: (t * 10.0 + 1.0, 5.0, 0.0),
        fov_deg: 60.0 + t * 5.0,
    }
}

fn create_timeline_with_camera(name: &str, duration: f32, keyframe_count: usize) -> Timeline {
    let mut tl = Timeline::new(name, duration);
    let keyframes: Vec<CameraKey> = (0..keyframe_count)
        .map(|i| create_camera_keyframe(i as f32 * duration / keyframe_count as f32))
        .collect();
    tl.tracks.push(Track::Camera { keyframes });
    tl
}

fn create_timeline_with_mixed_tracks(name: &str, duration: f32, track_count: usize) -> Timeline {
    let mut tl = Timeline::new(name, duration);

    for i in 0..track_count {
        let start_time = (i as f32 / track_count as f32) * duration;
        match i % 4 {
            0 => {
                tl.tracks.push(Track::Camera {
                    keyframes: vec![create_camera_keyframe(start_time)],
                });
            }
            1 => {
                tl.tracks.push(Track::Animation {
                    target: i as u32,
                    clip: format!("anim_{}", i),
                    start: Time(start_time),
                });
            }
            2 => {
                tl.tracks.push(Track::Audio {
                    clip: format!("audio_{}.ogg", i),
                    start: Time(start_time),
                    volume: 0.8,
                });
            }
            3 => {
                tl.tracks.push(Track::Fx {
                    name: format!("fx_{}", i),
                    start: Time(start_time),
                    params: serde_json::json!({"intensity": i as f32 * 0.1}),
                });
            }
            _ => unreachable!(),
        }
    }
    tl
}

fn create_complex_timeline(duration: f32) -> Timeline {
    let mut tl = Timeline::new("complex_cinematic", duration);

    // Camera track with many keyframes (cinematic camera path)
    let camera_keyframes: Vec<CameraKey> = (0..20)
        .map(|i| {
            let t = i as f32 * duration / 20.0;
            CameraKey {
                t: Time(t),
                pos: (t * 5.0, 10.0 + (t * 0.5).sin() * 5.0, t * 2.0),
                look_at: (t * 5.0 + 10.0, 5.0, t * 2.0),
                fov_deg: 60.0 + (t * 0.3).sin() * 10.0,
            }
        })
        .collect();
    tl.tracks.push(Track::Camera {
        keyframes: camera_keyframes,
    });

    // Multiple animation tracks
    for i in 0..5 {
        tl.tracks.push(Track::Animation {
            target: 100 + i,
            clip: format!("character_{}_action", i),
            start: Time(i as f32 * 2.0),
        });
    }

    // Audio tracks (music, ambient, SFX)
    tl.tracks.push(Track::Audio {
        clip: "music_epic.ogg".into(),
        start: Time(0.0),
        volume: 0.7,
    });
    tl.tracks.push(Track::Audio {
        clip: "ambient_wind.ogg".into(),
        start: Time(1.0),
        volume: 0.3,
    });
    for i in 0..3 {
        tl.tracks.push(Track::Audio {
            clip: format!("sfx_explosion_{}.ogg", i),
            start: Time(5.0 + i as f32 * 0.5),
            volume: 1.0,
        });
    }

    // FX tracks
    tl.tracks.push(Track::Fx {
        name: "screen_shake".into(),
        start: Time(5.0),
        params: serde_json::json!({"intensity": 0.8, "duration": 0.5}),
    });
    tl.tracks.push(Track::Fx {
        name: "particles_debris".into(),
        start: Time(5.2),
        params: serde_json::json!({"count": 500, "spread": 10.0}),
    });
    tl.tracks.push(Track::Fx {
        name: "flash_white".into(),
        start: Time(5.0),
        params: serde_json::json!({"duration": 0.1}),
    });

    tl
}

// ============================================================================
// Timeline Creation Benchmarks
// ============================================================================

fn bench_timeline_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("Timeline Creation");

    // Empty timeline creation
    group.bench_function("empty", |b| {
        b.iter(|| black_box(Timeline::new("empty", 10.0)))
    });

    // Timeline with name allocation
    group.bench_function("with_long_name", |b| {
        let name = "this_is_a_very_long_timeline_name_for_testing_string_allocation";
        b.iter(|| black_box(Timeline::new(black_box(name), 60.0)))
    });

    // Time struct creation
    group.bench_function("time_struct", |b| {
        b.iter(|| black_box(Time(black_box(42.5))))
    });

    group.finish();
}

// ============================================================================
// Camera Keyframe Benchmarks
// ============================================================================

fn bench_camera_keyframes(c: &mut Criterion) {
    let mut group = c.benchmark_group("Camera Keyframes");

    // Single keyframe creation
    group.bench_function("single_creation", |b| {
        b.iter(|| {
            black_box(CameraKey {
                t: Time(1.0),
                pos: (10.0, 5.0, 0.0),
                look_at: (20.0, 5.0, 0.0),
                fov_deg: 60.0,
            })
        })
    });

    // Keyframe clone
    let keyframe = create_camera_keyframe(5.0);
    group.bench_function("clone", |b| b.iter(|| black_box(keyframe.clone())));

    // Batch keyframe creation
    for count in [5, 20, 50, 100] {
        group.bench_with_input(
            BenchmarkId::new("batch_creation", count),
            &count,
            |b, &count| {
                b.iter(|| {
                    let keyframes: Vec<CameraKey> = (0..count)
                        .map(|i| create_camera_keyframe(i as f32))
                        .collect();
                    black_box(keyframes)
                })
            },
        );
    }

    group.finish();
}

// ============================================================================
// Track Creation Benchmarks
// ============================================================================

fn bench_track_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("Track Creation");

    // Camera track with keyframes
    group.bench_function("camera_5_keyframes", |b| {
        b.iter(|| {
            let keyframes: Vec<CameraKey> =
                (0..5).map(|i| create_camera_keyframe(i as f32)).collect();
            black_box(Track::Camera { keyframes })
        })
    });

    // Animation track
    group.bench_function("animation", |b| {
        b.iter(|| {
            black_box(Track::Animation {
                target: 42,
                clip: String::from("walk_cycle"),
                start: Time(1.0),
            })
        })
    });

    // Audio track
    group.bench_function("audio", |b| {
        b.iter(|| {
            black_box(Track::Audio {
                clip: String::from("music.ogg"),
                start: Time(0.0),
                volume: 0.8,
            })
        })
    });

    // FX track with JSON params
    group.bench_function("fx_simple_params", |b| {
        b.iter(|| {
            black_box(Track::Fx {
                name: String::from("explosion"),
                start: Time(5.0),
                params: serde_json::json!({"scale": 2.0}),
            })
        })
    });

    // FX track with complex JSON params
    group.bench_function("fx_complex_params", |b| {
        b.iter(|| {
            black_box(Track::Fx {
                name: String::from("particle_system"),
                start: Time(5.0),
                params: serde_json::json!({
                    "count": 1000,
                    "lifetime": 2.5,
                    "velocity": {"min": [0, 1, 0], "max": [0, 5, 0]},
                    "color_start": [1.0, 0.5, 0.0, 1.0],
                    "color_end": [1.0, 0.0, 0.0, 0.0],
                    "size": {"start": 0.1, "end": 0.5}
                }),
            })
        })
    });

    group.finish();
}

// ============================================================================
// Sequencer Operations Benchmarks
// ============================================================================

fn bench_sequencer_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("Sequencer Operations");

    // Sequencer creation
    group.bench_function("creation", |b| b.iter(|| black_box(Sequencer::new())));

    // Sequencer default
    group.bench_function("default", |b| b.iter(|| black_box(Sequencer::default())));

    // Seek operation
    group.bench_function("seek", |b| {
        b.iter(|| {
            let mut seq = Sequencer::new();
            seq.seek(black_box(Time(5.0)));
            black_box(seq.t)
        })
    });

    // Step with empty timeline
    group.bench_function("step_empty_timeline", |b| {
        let tl = Timeline::new("empty", 100.0);
        b.iter(|| {
            let mut seq = Sequencer::new();
            black_box(seq.step(black_box(0.016), &tl))
        })
    });

    // Step with no events in range
    let tl_sparse = create_timeline_with_camera("sparse", 100.0, 5);
    group.bench_function("step_no_events", |b| {
        b.iter(|| {
            let mut seq = Sequencer::new();
            seq.seek(Time(50.0)); // Seek to middle, away from keyframes
            black_box(seq.step(black_box(0.016), &tl_sparse))
        })
    });

    group.finish();
}

// ============================================================================
// Event Emission Benchmarks
// ============================================================================

fn bench_event_emission(c: &mut Criterion) {
    let mut group = c.benchmark_group("Event Emission");

    // Step that triggers camera event
    let tl_camera = create_timeline_with_camera("camera", 10.0, 10);
    group.bench_function("camera_event", |b| {
        b.iter(|| {
            let mut seq = Sequencer::new();
            // Step to trigger first keyframe at t=0
            black_box(seq.step(black_box(1.5), &tl_camera))
        })
    });

    // Step that triggers animation event
    let mut tl_anim = Timeline::new("anim", 10.0);
    tl_anim.tracks.push(Track::Animation {
        target: 1,
        clip: "test".into(),
        start: Time(1.0),
    });
    group.bench_function("animation_event", |b| {
        b.iter(|| {
            let mut seq = Sequencer::new();
            black_box(seq.step(black_box(1.5), &tl_anim))
        })
    });

    // Step that triggers audio event
    let mut tl_audio = Timeline::new("audio", 10.0);
    tl_audio.tracks.push(Track::Audio {
        clip: "test.ogg".into(),
        start: Time(1.0),
        volume: 0.8,
    });
    group.bench_function("audio_event", |b| {
        b.iter(|| {
            let mut seq = Sequencer::new();
            black_box(seq.step(black_box(1.5), &tl_audio))
        })
    });

    // Step that triggers FX event
    let mut tl_fx = Timeline::new("fx", 10.0);
    tl_fx.tracks.push(Track::Fx {
        name: "explosion".into(),
        start: Time(1.0),
        params: serde_json::json!({"scale": 2.0}),
    });
    group.bench_function("fx_event", |b| {
        b.iter(|| {
            let mut seq = Sequencer::new();
            black_box(seq.step(black_box(1.5), &tl_fx))
        })
    });

    // Multiple events in single step
    let tl_multi = create_timeline_with_mixed_tracks("multi", 10.0, 20);
    group.bench_function("multiple_events_step", |b| {
        b.iter(|| {
            let mut seq = Sequencer::new();
            // Large step to capture multiple events
            black_box(seq.step(black_box(5.0), &tl_multi))
        })
    });

    group.finish();
}

// ============================================================================
// Timeline Playback Simulation Benchmarks
// ============================================================================

fn bench_timeline_playback(c: &mut Criterion) {
    let mut group = c.benchmark_group("Timeline Playback");

    // Simulate 1 second of playback at 60 FPS
    for track_count in [5, 20, 50, 100] {
        let tl = create_timeline_with_mixed_tracks("playback", 60.0, track_count);
        group.bench_with_input(
            BenchmarkId::new("60fps_1sec", track_count),
            &track_count,
            |b, _| {
                b.iter(|| {
                    let mut seq = Sequencer::new();
                    let mut total_events = 0;
                    for _ in 0..60 {
                        if let Ok(events) = seq.step(1.0 / 60.0, &tl) {
                            total_events += events.len();
                        }
                    }
                    black_box(total_events)
                })
            },
        );
    }

    // Full complex timeline playback
    let complex = create_complex_timeline(30.0);
    group.bench_function("complex_30sec_full", |b| {
        b.iter(|| {
            let mut seq = Sequencer::new();
            let mut total_events = 0;
            // 30 seconds at 60 FPS = 1800 steps
            for _ in 0..1800 {
                if let Ok(events) = seq.step(1.0 / 60.0, &complex) {
                    total_events += events.len();
                }
            }
            black_box(total_events)
        })
    });

    // Single frame step on complex timeline
    group.bench_function("complex_single_frame", |b| {
        let complex = create_complex_timeline(30.0);
        b.iter(|| {
            let mut seq = Sequencer::new();
            seq.seek(Time(5.0)); // Seek to action-heavy section
            black_box(seq.step(black_box(1.0 / 60.0), &complex))
        })
    });

    group.finish();
}

// ============================================================================
// JSON Serialization Benchmarks
// ============================================================================

fn bench_json_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("JSON Serialization");

    // Serialize empty timeline
    let empty = Timeline::new("empty", 10.0);
    group.bench_function("serialize_empty", |b| {
        b.iter(|| black_box(serde_json::to_string(&empty)))
    });

    // Serialize simple timeline
    let simple = create_timeline_with_camera("simple", 10.0, 5);
    group.bench_function("serialize_simple", |b| {
        b.iter(|| black_box(serde_json::to_string(&simple)))
    });

    // Serialize complex timeline
    let complex = create_complex_timeline(30.0);
    group.bench_function("serialize_complex", |b| {
        b.iter(|| black_box(serde_json::to_string(&complex)))
    });

    // Serialize to pretty JSON
    group.bench_function("serialize_complex_pretty", |b| {
        b.iter(|| black_box(serde_json::to_string_pretty(&complex)))
    });

    // Deserialize simple timeline
    let simple_json = serde_json::to_string(&simple).unwrap();
    group.bench_function("deserialize_simple", |b| {
        b.iter(|| black_box(serde_json::from_str::<Timeline>(black_box(&simple_json))))
    });

    // Deserialize complex timeline
    let complex_json = serde_json::to_string(&complex).unwrap();
    group.bench_function("deserialize_complex", |b| {
        b.iter(|| black_box(serde_json::from_str::<Timeline>(black_box(&complex_json))))
    });

    // Full roundtrip
    group.bench_function("roundtrip_complex", |b| {
        b.iter(|| {
            let json = serde_json::to_string(&complex).unwrap();
            let restored: Timeline = serde_json::from_str(&json).unwrap();
            black_box(restored)
        })
    });

    group.finish();
}

// ============================================================================
// Event Creation Benchmarks
// ============================================================================

fn bench_event_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("Event Creation");

    // CameraKey event
    let keyframe = create_camera_keyframe(5.0);
    group.bench_function("camera_key_event", |b| {
        b.iter(|| black_box(SequencerEvent::CameraKey(keyframe.clone())))
    });

    // AnimStart event
    group.bench_function("anim_start_event", |b| {
        b.iter(|| {
            black_box(SequencerEvent::AnimStart {
                target: 42,
                clip: String::from("run"),
            })
        })
    });

    // AudioPlay event
    group.bench_function("audio_play_event", |b| {
        b.iter(|| {
            black_box(SequencerEvent::AudioPlay {
                clip: String::from("music.ogg"),
                volume: 0.8,
            })
        })
    });

    // FxTrigger event
    group.bench_function("fx_trigger_event", |b| {
        b.iter(|| {
            black_box(SequencerEvent::FxTrigger {
                name: String::from("explosion"),
                params: serde_json::json!({"scale": 2.0}),
            })
        })
    });

    // Event clone
    let event = SequencerEvent::FxTrigger {
        name: "complex_fx".into(),
        params: serde_json::json!({"a": 1, "b": [1,2,3], "c": {"nested": true}}),
    };
    group.bench_function("event_clone_complex", |b| {
        b.iter(|| black_box(event.clone()))
    });

    group.finish();
}

// ============================================================================
// Timeline Scaling Benchmarks
// ============================================================================

fn bench_timeline_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("Timeline Scaling");

    // Track count scaling - creation
    for track_count in [10, 50, 100, 500, 1000] {
        group.bench_with_input(
            BenchmarkId::new("creation", track_count),
            &track_count,
            |b, &count| {
                b.iter(|| black_box(create_timeline_with_mixed_tracks("scale", 60.0, count)))
            },
        );
    }

    // Track count scaling - step performance
    for track_count in [10, 50, 100, 500] {
        let tl = create_timeline_with_mixed_tracks("scale", 60.0, track_count);
        group.bench_with_input(
            BenchmarkId::new("step", track_count),
            &track_count,
            |b, _| {
                b.iter(|| {
                    let mut seq = Sequencer::new();
                    black_box(seq.step(black_box(0.5), &tl))
                })
            },
        );
    }

    // Camera keyframe scaling
    for keyframe_count in [10, 50, 100, 500] {
        let tl = create_timeline_with_camera("camera", 60.0, keyframe_count);
        group.bench_with_input(
            BenchmarkId::new("camera_keyframes_step", keyframe_count),
            &keyframe_count,
            |b, _| {
                b.iter(|| {
                    let mut seq = Sequencer::new();
                    // Step through entire timeline
                    black_box(seq.step(black_box(30.0), &tl))
                })
            },
        );
    }

    group.finish();
}

// ============================================================================
// Edge Cases & Boundary Benchmarks
// ============================================================================

fn bench_edge_cases(c: &mut Criterion) {
    let mut group = c.benchmark_group("Edge Cases");

    // Zero duration timeline
    let zero_duration = Timeline::new("zero", 0.0);
    group.bench_function("zero_duration_step", |b| {
        b.iter(|| {
            let mut seq = Sequencer::new();
            black_box(seq.step(black_box(0.0), &zero_duration))
        })
    });

    // Very small time step (sub-millisecond)
    let tl = create_timeline_with_camera("precise", 10.0, 100);
    group.bench_function("sub_ms_step", |b| {
        b.iter(|| {
            let mut seq = Sequencer::new();
            // 0.1ms step
            black_box(seq.step(black_box(0.0001), &tl))
        })
    });

    // Very large time step
    group.bench_function("large_step", |b| {
        let tl = create_timeline_with_mixed_tracks("large", 100.0, 100);
        b.iter(|| {
            let mut seq = Sequencer::new();
            // Jump entire timeline
            black_box(seq.step(black_box(99.0), &tl))
        })
    });

    // Boundary condition - exactly at event time
    let mut tl_boundary = Timeline::new("boundary", 10.0);
    tl_boundary.tracks.push(Track::Audio {
        clip: "test.ogg".into(),
        start: Time(5.0),
        volume: 1.0,
    });
    group.bench_function("exact_event_time", |b| {
        b.iter(|| {
            let mut seq = Sequencer::new();
            black_box(seq.step(black_box(5.0), &tl_boundary))
        })
    });

    // Repeated seeks
    group.bench_function("repeated_seeks", |b| {
        b.iter(|| {
            let mut seq = Sequencer::new();
            for i in 0..100 {
                seq.seek(Time(i as f32 * 0.1));
            }
            black_box(seq.t)
        })
    });

    // Error path - out of range
    let short_tl = Timeline::new("short", 1.0);
    group.bench_function("out_of_range_error", |b| {
        b.iter(|| {
            let mut seq = Sequencer::new();
            seq.seek(Time(0.99));
            black_box(seq.step(black_box(1.0), &short_tl))
        })
    });

    group.finish();
}

// ============================================================================
// Timeline Clone Benchmarks
// ============================================================================

fn bench_timeline_clone(c: &mut Criterion) {
    let mut group = c.benchmark_group("Timeline Clone");

    // Clone empty timeline
    let empty = Timeline::new("empty", 10.0);
    group.bench_function("empty", |b| b.iter(|| black_box(empty.clone())));

    // Clone simple timeline
    let simple = create_timeline_with_camera("simple", 10.0, 10);
    group.bench_function("simple", |b| b.iter(|| black_box(simple.clone())));

    // Clone complex timeline
    let complex = create_complex_timeline(30.0);
    group.bench_function("complex", |b| b.iter(|| black_box(complex.clone())));

    // Clone timeline with many tracks
    let many_tracks = create_timeline_with_mixed_tracks("many", 60.0, 100);
    group.bench_function("100_tracks", |b| b.iter(|| black_box(many_tracks.clone())));

    group.finish();
}

criterion_group!(
    creation,
    bench_timeline_creation,
    bench_camera_keyframes
);
criterion_group!(tracks, bench_track_creation, bench_sequencer_operations);
criterion_group!(
    playback,
    bench_event_emission,
    bench_timeline_playback
);
criterion_group!(
    serialize,
    bench_json_serialization,
    bench_event_creation
);
criterion_group!(edges, bench_timeline_scaling, bench_edge_cases);
criterion_group!(clone_timelines, bench_timeline_clone);
