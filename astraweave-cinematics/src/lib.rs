#![forbid(unsafe_code)]
use serde::{Deserialize, Serialize};

#[cfg(test)]
mod mutation_tests;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, PartialOrd, Default)]
pub struct Time(pub f32); // seconds

impl Time {
    /// Creates a new Time from seconds.
    #[inline]
    pub fn from_secs(secs: f32) -> Self {
        Self(secs)
    }

    /// Creates a new Time from milliseconds.
    #[inline]
    pub fn from_millis(millis: f32) -> Self {
        Self(millis / 1000.0)
    }

    /// Returns the time in seconds.
    #[inline]
    pub fn as_secs(&self) -> f32 {
        self.0
    }

    /// Returns the time in milliseconds.
    #[inline]
    pub fn as_millis(&self) -> f32 {
        self.0 * 1000.0
    }

    /// Returns zero time.
    #[inline]
    pub fn zero() -> Self {
        Self(0.0)
    }

    /// Returns true if this is zero time.
    #[inline]
    pub fn is_zero(&self) -> bool {
        self.0 == 0.0
    }

    /// Returns true if the time is positive.
    #[inline]
    pub fn is_positive(&self) -> bool {
        self.0 > 0.0
    }

    /// Adds the given seconds.
    #[inline]
    pub fn add_secs(&self, secs: f32) -> Self {
        Self(self.0 + secs)
    }

    /// Returns the time clamped to the given range.
    pub fn clamp(&self, min: Time, max: Time) -> Self {
        Self(self.0.clamp(min.0, max.0))
    }

    /// Linearly interpolates between two times.
    pub fn lerp(&self, other: Time, t: f32) -> Self {
        Self(self.0 + (other.0 - self.0) * t)
    }
}

impl std::fmt::Display for Time {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.0 >= 1.0 {
            write!(f, "{:.2}s", self.0)
        } else {
            write!(f, "{:.0}ms", self.0 * 1000.0)
        }
    }
}

impl std::ops::Add for Time {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self(self.0 + rhs.0)
    }
}

impl std::ops::Sub for Time {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self(self.0 - rhs.0)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[non_exhaustive]
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

impl Track {
    /// Returns the name of this track type.
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::Camera { .. } => "Camera",
            Self::Animation { .. } => "Animation",
            Self::Audio { .. } => "Audio",
            Self::Fx { .. } => "Fx",
        }
    }

    /// Returns true if this is a camera track.
    #[inline]
    pub fn is_camera(&self) -> bool {
        matches!(self, Self::Camera { .. })
    }

    /// Returns true if this is an animation track.
    #[inline]
    pub fn is_animation(&self) -> bool {
        matches!(self, Self::Animation { .. })
    }

    /// Returns true if this is an audio track.
    #[inline]
    pub fn is_audio(&self) -> bool {
        matches!(self, Self::Audio { .. })
    }

    /// Returns true if this is an FX track.
    #[inline]
    pub fn is_fx(&self) -> bool {
        matches!(self, Self::Fx { .. })
    }

    /// Returns the start time of this track (None for Camera tracks).
    pub fn start_time(&self) -> Option<Time> {
        match self {
            Self::Camera { .. } => None,
            Self::Animation { start, .. } => Some(*start),
            Self::Audio { start, .. } => Some(*start),
            Self::Fx { start, .. } => Some(*start),
        }
    }

    /// Returns the number of keyframes for camera tracks.
    pub fn keyframe_count(&self) -> Option<usize> {
        match self {
            Self::Camera { keyframes } => Some(keyframes.len()),
            _ => None,
        }
    }

    /// Creates a camera track with keyframes.
    pub fn camera(keyframes: Vec<CameraKey>) -> Self {
        Self::Camera { keyframes }
    }

    /// Creates an animation track.
    pub fn animation(target: u32, clip: impl Into<String>, start: Time) -> Self {
        Self::Animation {
            target,
            clip: clip.into(),
            start,
        }
    }

    /// Creates an audio track.
    pub fn audio(clip: impl Into<String>, start: Time, volume: f32) -> Self {
        Self::Audio {
            clip: clip.into(),
            start,
            volume,
        }
    }

    /// Creates an FX track.
    pub fn fx(name: impl Into<String>, start: Time, params: serde_json::Value) -> Self {
        Self::Fx {
            name: name.into(),
            start,
            params,
        }
    }
}

impl std::fmt::Display for Track {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Camera { keyframes } => write!(f, "Track::Camera({} keyframes)", keyframes.len()),
            Self::Animation {
                target,
                clip,
                start,
            } => {
                write!(
                    f,
                    "Track::Animation(target={}, clip=\"{}\", start={})",
                    target, clip, start
                )
            }
            Self::Audio {
                clip,
                start,
                volume,
            } => {
                write!(
                    f,
                    "Track::Audio(clip=\"{}\", start={}, volume={:.2})",
                    clip, start, volume
                )
            }
            Self::Fx { name, start, .. } => {
                write!(f, "Track::Fx(name=\"{}\", start={})", name, start)
            }
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CameraKey {
    pub t: Time,
    pub pos: (f32, f32, f32),
    pub look_at: (f32, f32, f32),
    pub fov_deg: f32,
}

impl CameraKey {
    /// Creates a new camera keyframe.
    pub fn new(t: Time, pos: (f32, f32, f32), look_at: (f32, f32, f32), fov_deg: f32) -> Self {
        Self {
            t,
            pos,
            look_at,
            fov_deg,
        }
    }

    /// Creates a camera key at time 0.
    pub fn at_origin(fov_deg: f32) -> Self {
        Self {
            t: Time::zero(),
            pos: (0.0, 0.0, 0.0),
            look_at: (0.0, 0.0, -1.0),
            fov_deg,
        }
    }

    /// Returns the position as a tuple.
    #[inline]
    pub fn position(&self) -> (f32, f32, f32) {
        self.pos
    }

    /// Returns the distance from position to look_at target.
    pub fn distance_to_target(&self) -> f32 {
        let dx = self.look_at.0 - self.pos.0;
        let dy = self.look_at.1 - self.pos.1;
        let dz = self.look_at.2 - self.pos.2;
        (dx * dx + dy * dy + dz * dz).sqrt()
    }

    /// Returns the FOV in radians.
    #[inline]
    pub fn fov_rad(&self) -> f32 {
        self.fov_deg.to_radians()
    }

    /// Clamp invalid configurations to valid ones.
    ///
    /// Mirrors [`astraweave_camera::FreeFly::sanitize`] (Unified Camera
    /// campaign sub-phase C.6.F) at the cinematics-keyframe layer.
    /// Callers invoke explicitly when they may have received pathological
    /// input (deserialized timelines, programmatically-generated keyframes,
    /// untrusted scene data). Additionally,
    /// `astraweave_render::Renderer::apply_camera_key` invokes this
    /// defensively before applying a key (Unified Camera C.7.D), so the
    /// production cinematics path is hardened without requiring every
    /// caller to remember to sanitize.
    ///
    /// Specifically, `sanitize()` ensures:
    ///
    /// - `self.fov_deg` is in `[10.0, 170.0]` degrees. Matches FreeFly's
    ///   `[10°, 170°]` range from C.6.F (the canonical convention range
    ///   per `CAMERA_CONVENTIONS.md` §2.1's typical FOV expectations).
    ///   Pre-C.7.D this crate had `is_typical_fov` returning a bool for
    ///   the tighter `30°..=120°` range but it was documentation-only
    ///   (zero callers); `sanitize()` replaces it with the wider
    ///   canonical range to harmonize with FreeFly.
    /// - `self.look_at != self.pos`: if they are exactly equal
    ///   (degenerate look-direction with no defined direction), `look_at`
    ///   is set to `pos + (1.0, 0.0, 0.0)` (canonical +X forward per
    ///   `CAMERA_CONVENTIONS.md` §2.8). Resolves the silent-degenerate-
    ///   acceptance bug (C.5 audit finding L.5.17) deterministically
    ///   rather than relying on
    ///   `astraweave_render::Renderer::apply_camera_key`'s implicit
    ///   `normalize_or_zero` → `atan2(0,0) = 0` fallback. Exact equality
    ///   only — near-degenerate cases (look_at ≈ pos but not exactly
    ///   equal) are out of scope per the C.7.D planning round's locked
    ///   decision.
    ///
    /// Pos and t are never modified.
    ///
    /// Added in Unified Camera campaign sub-phase C.7.D per C.5 audit
    /// finding L.5.17.
    pub fn sanitize(&mut self) {
        self.fov_deg = self.fov_deg.clamp(10.0, 170.0);
        if self.look_at == self.pos {
            self.look_at = (self.pos.0 + 1.0, self.pos.1, self.pos.2);
        }
    }

    /// Linearly interpolates between two camera keys.
    pub fn lerp(&self, other: &Self, t: f32) -> Self {
        Self {
            t: self.t.lerp(other.t, t),
            pos: (
                self.pos.0 + (other.pos.0 - self.pos.0) * t,
                self.pos.1 + (other.pos.1 - self.pos.1) * t,
                self.pos.2 + (other.pos.2 - self.pos.2) * t,
            ),
            look_at: (
                self.look_at.0 + (other.look_at.0 - self.look_at.0) * t,
                self.look_at.1 + (other.look_at.1 - self.look_at.1) * t,
                self.look_at.2 + (other.look_at.2 - self.look_at.2) * t,
            ),
            fov_deg: self.fov_deg + (other.fov_deg - self.fov_deg) * t,
        }
    }
}

impl std::fmt::Display for CameraKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "CameraKey(t={}, pos=({:.1},{:.1},{:.1}), fov={}°)",
            self.t, self.pos.0, self.pos.1, self.pos.2, self.fov_deg
        )
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
pub struct Timeline {
    pub name: String,
    pub duration: Time,
    pub tracks: Vec<Track>,
}

impl Timeline {
    /// Creates a new timeline with the given name and duration.
    pub fn new(name: &str, duration: f32) -> Self {
        Self {
            name: name.into(),
            duration: Time(duration),
            tracks: Vec::new(),
        }
    }

    /// Creates an empty timeline.
    pub fn empty() -> Self {
        Self::default()
    }

    /// Returns true if the timeline has no tracks.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.tracks.is_empty()
    }

    /// Returns the number of tracks.
    #[inline]
    pub fn track_count(&self) -> usize {
        self.tracks.len()
    }

    /// Returns the number of camera tracks.
    pub fn camera_track_count(&self) -> usize {
        self.tracks.iter().filter(|t| t.is_camera()).count()
    }

    /// Returns the number of audio tracks.
    pub fn audio_track_count(&self) -> usize {
        self.tracks.iter().filter(|t| t.is_audio()).count()
    }

    /// Returns the number of animation tracks.
    pub fn animation_track_count(&self) -> usize {
        self.tracks.iter().filter(|t| t.is_animation()).count()
    }

    /// Returns the number of FX tracks.
    pub fn fx_track_count(&self) -> usize {
        self.tracks.iter().filter(|t| t.is_fx()).count()
    }

    /// Adds a track to the timeline.
    pub fn add_track(&mut self, track: Track) {
        self.tracks.push(track);
    }

    /// Adds a camera track with keyframes.
    pub fn add_camera_track(&mut self, keyframes: Vec<CameraKey>) {
        self.tracks.push(Track::Camera { keyframes });
    }

    /// Adds an audio track.
    pub fn add_audio_track(&mut self, clip: impl Into<String>, start: Time, volume: f32) {
        self.tracks.push(Track::Audio {
            clip: clip.into(),
            start,
            volume,
        });
    }

    /// Returns the total number of camera keyframes across all camera tracks.
    pub fn total_keyframes(&self) -> usize {
        self.tracks.iter().filter_map(|t| t.keyframe_count()).sum()
    }

    /// Returns the duration in seconds.
    #[inline]
    pub fn duration_secs(&self) -> f32 {
        self.duration.0
    }
}

impl std::fmt::Display for Timeline {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Timeline(\"{}\", duration={}, {} tracks)",
            self.name,
            self.duration,
            self.tracks.len()
        )
    }
}

#[derive(thiserror::Error, Debug)]
#[non_exhaustive]
#[must_use]
pub enum SeqError {
    #[error("timeline out of range: {0:?}")]
    Range(Time),
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
    pub fn step(&mut self, dt: f32, tl: &Timeline) -> Result<Vec<SequencerEvent>, SeqError> {
        let next_t = Time(self.t.0 + dt);
        if next_t.0 > tl.duration.0 + 0.001 {
            return Err(SeqError::Range(next_t));
        }
        let from = self.t.0;
        let to = next_t.0;
        self.t = next_t;
        // very simple: emit events whose start is in (from..=to]
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

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[non_exhaustive]
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

impl SequencerEvent {
    /// Returns the type name of this event.
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::CameraKey(_) => "CameraKey",
            Self::AnimStart { .. } => "AnimStart",
            Self::AudioPlay { .. } => "AudioPlay",
            Self::FxTrigger { .. } => "FxTrigger",
        }
    }

    /// Returns `true` if this is a camera key event.
    pub fn is_camera_key(&self) -> bool {
        matches!(self, Self::CameraKey(_))
    }

    /// Returns `true` if this is an animation start event.
    pub fn is_anim_start(&self) -> bool {
        matches!(self, Self::AnimStart { .. })
    }

    /// Returns `true` if this is an audio play event.
    pub fn is_audio_play(&self) -> bool {
        matches!(self, Self::AudioPlay { .. })
    }

    /// Returns `true` if this is an FX trigger event.
    pub fn is_fx_trigger(&self) -> bool {
        matches!(self, Self::FxTrigger { .. })
    }

    /// Returns the camera key if this is a camera key event.
    pub fn as_camera_key(&self) -> Option<&CameraKey> {
        match self {
            Self::CameraKey(k) => Some(k),
            _ => None,
        }
    }

    /// Returns the animation clip name if this is an animation start event.
    pub fn animation_clip(&self) -> Option<&str> {
        match self {
            Self::AnimStart { clip, .. } => Some(clip),
            _ => None,
        }
    }

    /// Returns the audio clip name if this is an audio play event.
    pub fn audio_clip(&self) -> Option<&str> {
        match self {
            Self::AudioPlay { clip, .. } => Some(clip),
            _ => None,
        }
    }

    /// Returns the FX name if this is an FX trigger event.
    pub fn fx_name(&self) -> Option<&str> {
        match self {
            Self::FxTrigger { name, .. } => Some(name),
            _ => None,
        }
    }

    /// Creates a camera key event.
    pub fn camera_key(key: CameraKey) -> Self {
        Self::CameraKey(key)
    }

    /// Creates an animation start event.
    pub fn anim_start(target: u32, clip: impl Into<String>) -> Self {
        Self::AnimStart {
            target,
            clip: clip.into(),
        }
    }

    /// Creates an audio play event with the specified clip and volume.
    pub fn audio_play(clip: impl Into<String>, volume: f32) -> Self {
        Self::AudioPlay {
            clip: clip.into(),
            volume,
        }
    }

    /// Creates an FX trigger event with the specified name and params.
    pub fn fx_trigger(name: impl Into<String>, params: serde_json::Value) -> Self {
        Self::FxTrigger {
            name: name.into(),
            params,
        }
    }
}

impl std::fmt::Display for SequencerEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CameraKey(k) => write!(f, "CameraKey({})", k),
            Self::AnimStart { target, clip } => {
                write!(f, "AnimStart(target={}, clip=\"{}\")", target, clip)
            }
            Self::AudioPlay { clip, volume } => {
                write!(f, "AudioPlay(clip=\"{}\", volume={:.2})", clip, volume)
            }
            Self::FxTrigger { name, .. } => write!(f, "FxTrigger(name=\"{}\")", name),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn seq_emits_events() {
        let mut tl = Timeline::new("demo", 3.0);
        tl.tracks.push(Track::Camera {
            keyframes: vec![CameraKey {
                t: Time(1.0),
                pos: (0.0, 1.0, 2.0),
                look_at: (0.0, 0.0, 0.0),
                fov_deg: 60.0,
            }],
        });
        tl.tracks.push(Track::Audio {
            clip: "boom".into(),
            start: Time(2.0),
            volume: 0.8,
        });
        let mut seq = Sequencer::new();
        let evs0 = seq.step(0.5, &tl).unwrap();
        assert!(evs0.is_empty());
        let evs1 = seq.step(0.6, &tl).unwrap();
        assert!(matches!(evs1[0], SequencerEvent::CameraKey(_)));
        let evs2 = seq.step(0.9, &tl).unwrap();
        assert!(matches!(evs2[0], SequencerEvent::AudioPlay { .. }));
        let evs3 = seq.step(1.0, &tl).unwrap();
        assert!(evs3.is_empty());
    }

    #[test]
    fn timeline_json_roundtrip() {
        let mut tl = Timeline::new("roundtrip", 2.0);
        tl.tracks.push(Track::Camera {
            keyframes: vec![CameraKey {
                t: Time(0.5),
                pos: (1.0, 2.0, 3.0),
                look_at: (0.0, 1.0, 0.0),
                fov_deg: 70.0,
            }],
        });
        tl.tracks.push(Track::Fx {
            name: "fade-in".into(),
            start: Time(0.0),
            params: serde_json::json!({"duration": 0.25}),
        });
        let s = serde_json::to_string_pretty(&tl).unwrap();
        let de: Timeline = serde_json::from_str(&s).unwrap();
        assert_eq!(tl, de);
    }

    #[test]
    fn sequencer_default() {
        // Test Default implementation
        let seq: Sequencer = Default::default();
        assert_eq!(seq.t.0, 0.0);
    }

    #[test]
    fn sequencer_seek() {
        let mut seq = Sequencer::new();
        assert_eq!(seq.t.0, 0.0);

        seq.seek(Time(5.5));
        assert_eq!(seq.t.0, 5.5);

        seq.seek(Time(0.0));
        assert_eq!(seq.t.0, 0.0);
    }

    #[test]
    fn sequencer_out_of_range_error() {
        let tl = Timeline::new("short", 1.0);
        let mut seq = Sequencer::new();

        // Step past the end
        let result = seq.step(2.0, &tl);
        assert!(result.is_err());

        // Check error message
        let err = result.unwrap_err();
        let msg = format!("{}", err);
        assert!(msg.contains("out of range"));
    }

    #[test]
    fn seq_emits_animation_events() {
        let mut tl = Timeline::new("anim_test", 3.0);
        tl.tracks.push(Track::Animation {
            target: 42,
            clip: "walk_cycle".into(),
            start: Time(1.0),
        });

        let mut seq = Sequencer::new();

        // Before animation start
        let evs0 = seq.step(0.5, &tl).unwrap();
        assert!(evs0.is_empty());

        // At animation start
        let evs1 = seq.step(0.6, &tl).unwrap();
        assert_eq!(evs1.len(), 1);
        match &evs1[0] {
            SequencerEvent::AnimStart { target, clip } => {
                assert_eq!(*target, 42);
                assert_eq!(clip, "walk_cycle");
            }
            _ => panic!("Expected AnimStart event"),
        }
    }

    #[test]
    fn seq_emits_fx_events() {
        let mut tl = Timeline::new("fx_test", 3.0);
        tl.tracks.push(Track::Fx {
            name: "explosion".into(),
            start: Time(1.5),
            params: serde_json::json!({"scale": 2.0, "color": "red"}),
        });

        let mut seq = Sequencer::new();

        // Before FX trigger
        let evs0 = seq.step(1.0, &tl).unwrap();
        assert!(evs0.is_empty());

        // At FX trigger
        let evs1 = seq.step(0.6, &tl).unwrap();
        assert_eq!(evs1.len(), 1);
        match &evs1[0] {
            SequencerEvent::FxTrigger { name, params } => {
                assert_eq!(name, "explosion");
                assert_eq!(params["scale"], 2.0);
                assert_eq!(params["color"], "red");
            }
            _ => panic!("Expected FxTrigger event"),
        }
    }

    #[test]
    fn seq_multiple_events_same_frame() {
        let mut tl = Timeline::new("multi", 2.0);
        // All events at t=1.0
        tl.tracks.push(Track::Camera {
            keyframes: vec![CameraKey {
                t: Time(1.0),
                pos: (0.0, 0.0, 0.0),
                look_at: (1.0, 0.0, 0.0),
                fov_deg: 90.0,
            }],
        });
        tl.tracks.push(Track::Audio {
            clip: "sound".into(),
            start: Time(1.0),
            volume: 1.0,
        });
        tl.tracks.push(Track::Animation {
            target: 1,
            clip: "run".into(),
            start: Time(1.0),
        });

        let mut seq = Sequencer::new();
        let evs = seq.step(1.5, &tl).unwrap();

        // Should have all 3 events
        assert_eq!(evs.len(), 3);
    }

    #[test]
    fn timeline_empty_tracks() {
        let tl = Timeline::new("empty", 5.0);
        let mut seq = Sequencer::new();

        let evs = seq.step(1.0, &tl).unwrap();
        assert!(evs.is_empty());

        let evs = seq.step(2.0, &tl).unwrap();
        assert!(evs.is_empty());
    }

    #[test]
    fn camera_multiple_keyframes() {
        let mut tl = Timeline::new("camera_test", 5.0);
        tl.tracks.push(Track::Camera {
            keyframes: vec![
                CameraKey {
                    t: Time(1.0),
                    pos: (0.0, 0.0, 0.0),
                    look_at: (1.0, 0.0, 0.0),
                    fov_deg: 60.0,
                },
                CameraKey {
                    t: Time(2.0),
                    pos: (5.0, 0.0, 0.0),
                    look_at: (6.0, 0.0, 0.0),
                    fov_deg: 70.0,
                },
                CameraKey {
                    t: Time(3.0),
                    pos: (10.0, 0.0, 0.0),
                    look_at: (11.0, 0.0, 0.0),
                    fov_deg: 80.0,
                },
            ],
        });

        let mut seq = Sequencer::new();

        // Hit first keyframe
        let evs1 = seq.step(1.5, &tl).unwrap();
        assert_eq!(evs1.len(), 1);

        // Hit second keyframe
        let evs2 = seq.step(1.0, &tl).unwrap();
        assert_eq!(evs2.len(), 1);

        // Hit third keyframe
        let evs3 = seq.step(1.0, &tl).unwrap();
        assert_eq!(evs3.len(), 1);
    }

    #[test]
    fn time_struct() {
        // Test Time struct directly
        let t1 = Time(1.5);
        let t2 = Time(1.5);
        let t3 = Time(2.0);

        assert_eq!(t1, t2);
        assert_ne!(t1, t3);
        assert_eq!(t1.0, 1.5);
    }

    #[test]
    fn track_variants_equality() {
        // Test track equality
        let camera1 = Track::Camera {
            keyframes: vec![CameraKey {
                t: Time(1.0),
                pos: (0.0, 0.0, 0.0),
                look_at: (1.0, 0.0, 0.0),
                fov_deg: 60.0,
            }],
        };
        let camera2 = Track::Camera {
            keyframes: vec![CameraKey {
                t: Time(1.0),
                pos: (0.0, 0.0, 0.0),
                look_at: (1.0, 0.0, 0.0),
                fov_deg: 60.0,
            }],
        };
        assert_eq!(camera1, camera2);

        let anim = Track::Animation {
            target: 1,
            clip: "test".into(),
            start: Time(0.0),
        };
        assert_ne!(camera1, anim);
    }

    #[test]
    fn sequencer_event_equality() {
        let ev1 = SequencerEvent::AudioPlay {
            clip: "test".into(),
            volume: 0.5,
        };
        let ev2 = SequencerEvent::AudioPlay {
            clip: "test".into(),
            volume: 0.5,
        };
        let ev3 = SequencerEvent::AudioPlay {
            clip: "other".into(),
            volume: 0.5,
        };

        assert_eq!(ev1, ev2);
        assert_ne!(ev1, ev3);
    }

    #[test]
    fn sequencer_boundary_conditions() {
        let tl = Timeline::new("boundary", 1.0);
        let mut seq = Sequencer::new();

        // Step exactly to the end (should work due to 0.001 tolerance)
        let result = seq.step(1.0, &tl);
        assert!(result.is_ok());

        // Step tiny bit more
        let result2 = seq.step(0.0005, &tl);
        assert!(result2.is_ok()); // Within tolerance

        // Step definitely past
        let result3 = seq.step(0.01, &tl);
        assert!(result3.is_err());
    }

    #[test]
    fn seq_error_display() {
        let err = SeqError::Range(Time(5.5));
        let msg = format!("{}", err);
        assert!(msg.contains("5.5"));
        assert!(msg.contains("out of range"));
    }

    #[test]
    fn camera_key_equality() {
        let key1 = CameraKey {
            t: Time(1.0),
            pos: (1.0, 2.0, 3.0),
            look_at: (0.0, 0.0, 0.0),
            fov_deg: 60.0,
        };
        let key2 = CameraKey {
            t: Time(1.0),
            pos: (1.0, 2.0, 3.0),
            look_at: (0.0, 0.0, 0.0),
            fov_deg: 60.0,
        };
        let key3 = CameraKey {
            t: Time(2.0),
            pos: (1.0, 2.0, 3.0),
            look_at: (0.0, 0.0, 0.0),
            fov_deg: 60.0,
        };

        assert_eq!(key1, key2);
        assert_ne!(key1, key3);
    }

    #[test]
    fn timeline_with_all_track_types_roundtrip() {
        let mut tl = Timeline::new("full", 10.0);
        tl.tracks.push(Track::Camera {
            keyframes: vec![
                CameraKey {
                    t: Time(0.0),
                    pos: (0.0, 0.0, 0.0),
                    look_at: (1.0, 0.0, 0.0),
                    fov_deg: 60.0,
                },
                CameraKey {
                    t: Time(5.0),
                    pos: (5.0, 0.0, 0.0),
                    look_at: (6.0, 0.0, 0.0),
                    fov_deg: 90.0,
                },
            ],
        });
        tl.tracks.push(Track::Animation {
            target: 100,
            clip: "dance".into(),
            start: Time(2.0),
        });
        tl.tracks.push(Track::Audio {
            clip: "music.ogg".into(),
            start: Time(0.0),
            volume: 0.75,
        });
        tl.tracks.push(Track::Fx {
            name: "particles".into(),
            start: Time(3.0),
            params: serde_json::json!({"count": 1000}),
        });

        let json = serde_json::to_string(&tl).unwrap();
        let restored: Timeline = serde_json::from_str(&json).unwrap();
        assert_eq!(tl, restored);
        assert_eq!(restored.tracks.len(), 4);
    }

    // ============================================================
    // Time helper tests
    // ============================================================

    #[test]
    fn test_time_from_secs() {
        let t = Time::from_secs(2.5);
        assert!((t.0 - 2.5).abs() < 0.0001);
    }

    #[test]
    fn test_time_from_millis() {
        let t = Time::from_millis(1500.0);
        assert!((t.0 - 1.5).abs() < 0.0001);
    }

    #[test]
    fn test_time_as_secs() {
        let t = Time(3.25);
        assert!((t.as_secs() - 3.25).abs() < 0.0001);
    }

    #[test]
    fn test_time_as_millis() {
        let t = Time(2.5);
        assert!((t.as_millis() - 2500.0).abs() < 0.001);
    }

    #[test]
    fn test_time_zero() {
        let t = Time::zero();
        assert!((t.0).abs() < 0.0001);
        assert!(t.is_zero());
    }

    #[test]
    fn test_time_is_zero() {
        assert!(Time(0.0).is_zero());
        assert!(!Time(0.1).is_zero());
        assert!(!Time(0.0001).is_zero()); // Uses exact comparison
    }

    #[test]
    fn test_time_is_positive() {
        assert!(Time(1.0).is_positive());
        assert!(!Time(0.0).is_positive());
        assert!(!Time(-1.0).is_positive());
    }

    #[test]
    fn test_time_add_secs() {
        let t = Time(1.0);
        let t2 = t.add_secs(0.5);
        assert!((t2.0 - 1.5).abs() < 0.0001);
    }

    #[test]
    fn test_time_clamp() {
        let t = Time(5.0);
        let clamped = t.clamp(Time(2.0), Time(4.0));
        assert!((clamped.0 - 4.0).abs() < 0.0001);

        let t2 = Time(1.0);
        let clamped2 = t2.clamp(Time(2.0), Time(4.0));
        assert!((clamped2.0 - 2.0).abs() < 0.0001);

        let t3 = Time(3.0);
        let clamped3 = t3.clamp(Time(2.0), Time(4.0));
        assert!((clamped3.0 - 3.0).abs() < 0.0001);
    }

    #[test]
    fn test_time_lerp() {
        let start = Time(0.0);
        let end = Time(10.0);
        let mid = start.lerp(end, 0.5);
        assert!((mid.0 - 5.0).abs() < 0.0001);

        let quarter = start.lerp(end, 0.25);
        assert!((quarter.0 - 2.5).abs() < 0.0001);
    }

    #[test]
    fn test_time_add_op() {
        let a = Time(1.0);
        let b = Time(2.0);
        let c = a + b;
        assert!((c.0 - 3.0).abs() < 0.0001);
    }

    #[test]
    fn test_time_sub_op() {
        let a = Time(5.0);
        let b = Time(2.0);
        let c = a - b;
        assert!((c.0 - 3.0).abs() < 0.0001);
    }

    #[test]
    fn test_time_display() {
        let t = Time(3.5);
        let s = format!("{}", t);
        assert!(s.contains("3.50"));
    }

    #[test]
    fn test_time_partial_ord() {
        let a = Time(1.0);
        let b = Time(2.0);
        assert!(a < b);
        assert!(b > a);
        assert!(a <= a);
    }

    #[test]
    fn test_time_default() {
        let t = Time::default();
        assert!(t.is_zero());
    }

    // ============================================================
    // Track helper tests
    // ============================================================

    #[test]
    fn test_track_type_name() {
        let cam = Track::Camera { keyframes: vec![] };
        assert_eq!(cam.type_name(), "Camera");

        let anim = Track::Animation {
            target: 1,
            clip: "run".into(),
            start: Time(0.0),
        };
        assert_eq!(anim.type_name(), "Animation");

        let audio = Track::Audio {
            clip: "music".into(),
            start: Time(0.0),
            volume: 1.0,
        };
        assert_eq!(audio.type_name(), "Audio");

        let fx = Track::Fx {
            name: "explosion".into(),
            start: Time(0.0),
            params: serde_json::json!({}),
        };
        assert_eq!(fx.type_name(), "Fx");
    }

    #[test]
    fn test_track_is_camera() {
        let cam = Track::Camera { keyframes: vec![] };
        assert!(cam.is_camera());

        let audio = Track::Audio {
            clip: "music".into(),
            start: Time(0.0),
            volume: 1.0,
        };
        assert!(!audio.is_camera());
    }

    #[test]
    fn test_track_is_animation() {
        let anim = Track::Animation {
            target: 1,
            clip: "run".into(),
            start: Time(0.0),
        };
        assert!(anim.is_animation());

        let cam = Track::Camera { keyframes: vec![] };
        assert!(!cam.is_animation());
    }

    #[test]
    fn test_track_is_audio() {
        let audio = Track::Audio {
            clip: "music".into(),
            start: Time(0.0),
            volume: 1.0,
        };
        assert!(audio.is_audio());

        let anim = Track::Animation {
            target: 1,
            clip: "run".into(),
            start: Time(0.0),
        };
        assert!(!anim.is_audio());
    }

    #[test]
    fn test_track_is_fx() {
        let fx = Track::Fx {
            name: "explosion".into(),
            start: Time(0.0),
            params: serde_json::json!({}),
        };
        assert!(fx.is_fx());

        let audio = Track::Audio {
            clip: "music".into(),
            start: Time(0.0),
            volume: 1.0,
        };
        assert!(!audio.is_fx());
    }

    #[test]
    fn test_track_start_time() {
        let cam = Track::Camera {
            keyframes: vec![CameraKey {
                t: Time(1.0),
                pos: (0.0, 0.0, 0.0),
                look_at: (1.0, 0.0, 0.0),
                fov_deg: 60.0,
            }],
        };
        // Camera tracks return None for start_time
        assert!(cam.start_time().is_none());

        let anim = Track::Animation {
            target: 1,
            clip: "run".into(),
            start: Time(2.5),
        };
        assert!((anim.start_time().unwrap().0 - 2.5).abs() < 0.0001);

        let audio = Track::Audio {
            clip: "music".into(),
            start: Time(3.0),
            volume: 1.0,
        };
        assert!((audio.start_time().unwrap().0 - 3.0).abs() < 0.0001);

        let fx = Track::Fx {
            name: "explosion".into(),
            start: Time(4.0),
            params: serde_json::json!({}),
        };
        assert!((fx.start_time().unwrap().0 - 4.0).abs() < 0.0001);
    }

    #[test]
    fn test_track_keyframe_count() {
        let cam = Track::Camera {
            keyframes: vec![
                CameraKey {
                    t: Time(0.0),
                    pos: (0.0, 0.0, 0.0),
                    look_at: (1.0, 0.0, 0.0),
                    fov_deg: 60.0,
                },
                CameraKey {
                    t: Time(1.0),
                    pos: (1.0, 0.0, 0.0),
                    look_at: (2.0, 0.0, 0.0),
                    fov_deg: 60.0,
                },
            ],
        };
        assert_eq!(cam.keyframe_count(), Some(2));

        let anim = Track::Animation {
            target: 1,
            clip: "run".into(),
            start: Time(0.0),
        };
        // Non-camera tracks return None for keyframe_count
        assert_eq!(anim.keyframe_count(), None);
    }

    #[test]
    fn test_track_factory_camera() {
        let keyframes = vec![CameraKey {
            t: Time(0.0),
            pos: (0.0, 0.0, 0.0),
            look_at: (1.0, 0.0, 0.0),
            fov_deg: 60.0,
        }];
        let track = Track::camera(keyframes.clone());
        assert!(track.is_camera());
        assert_eq!(track.keyframe_count(), Some(1));
    }

    #[test]
    fn test_track_factory_animation() {
        let track = Track::animation(42, "walk", Time(1.5));
        assert!(track.is_animation());
        assert!((track.start_time().unwrap().0 - 1.5).abs() < 0.0001);
    }

    #[test]
    fn test_track_factory_audio() {
        let track = Track::audio("music.ogg", Time(0.0), 0.8);
        assert!(track.is_audio());
    }

    #[test]
    fn test_track_factory_fx() {
        let track = Track::fx("particles", Time(2.0), serde_json::json!({"count": 100}));
        assert!(track.is_fx());
        assert!((track.start_time().unwrap().0 - 2.0).abs() < 0.0001);
    }

    #[test]
    fn test_track_display() {
        let cam = Track::Camera { keyframes: vec![] };
        let s = format!("{}", cam);
        assert!(s.contains("Camera"));
        assert!(s.contains("0 keyframes"));

        let anim = Track::animation(10, "run", Time(1.0));
        let s2 = format!("{}", anim);
        assert!(s2.contains("Animation"));
        assert!(s2.contains("target=10"));
        assert!(s2.contains("run"));
    }

    // ============================================================
    // CameraKey helper tests
    // ============================================================

    #[test]
    fn test_camera_key_new() {
        let key = CameraKey::new(Time(1.0), (1.0, 2.0, 3.0), (0.0, 0.0, 0.0), 60.0);
        assert!((key.t.0 - 1.0).abs() < 0.0001);
        assert!((key.pos.0 - 1.0).abs() < 0.0001);
        assert!((key.fov_deg - 60.0).abs() < 0.0001);
    }

    #[test]
    fn test_camera_key_at_origin() {
        let key = CameraKey::at_origin(60.0);
        assert!((key.pos.0).abs() < 0.0001);
        assert!((key.pos.1).abs() < 0.0001);
        assert!((key.pos.2).abs() < 0.0001);
        assert!((key.fov_deg - 60.0).abs() < 0.0001);
    }

    #[test]
    fn test_camera_key_position() {
        let key = CameraKey::new(Time(0.0), (1.0, 2.0, 3.0), (0.0, 0.0, 0.0), 60.0);
        let pos = key.position();
        assert!((pos.0 - 1.0).abs() < 0.0001);
        assert!((pos.1 - 2.0).abs() < 0.0001);
        assert!((pos.2 - 3.0).abs() < 0.0001);
    }

    #[test]
    fn test_camera_key_distance_to_target() {
        let key = CameraKey::new(Time(0.0), (3.0, 0.0, 4.0), (0.0, 0.0, 0.0), 60.0);
        let dist = key.distance_to_target();
        assert!((dist - 5.0).abs() < 0.0001); // 3-4-5 triangle
    }

    #[test]
    fn test_camera_key_fov_rad() {
        let key = CameraKey::new(Time(0.0), (0.0, 0.0, 0.0), (0.0, 0.0, 0.0), 90.0);
        let rad = key.fov_rad();
        assert!((rad - std::f32::consts::FRAC_PI_2).abs() < 0.0001);
    }

    // C.7.D: `test_camera_key_is_typical_fov` removed alongside the
    // `is_typical_fov` method (zero production callers; replaced by
    // `sanitize()` at the wider [10°, 170°] canonical range per Q6 of
    // the C.7.D planning round). The new `sanitize_*` tests below cover
    // the post-C.7.D behavior; they mirror C.6.F's six FreeFly sanitize
    // tests adapted for CameraKey's specifics (tuple storage, no aspect
    // field, exact-equal degenerate look_at check).

    #[test]
    fn sanitize_clamps_fov_deg_below_minimum() {
        let mut key = CameraKey::new(Time(0.0), (0.0, 0.0, 0.0), (1.0, 0.0, 0.0), 5.0);
        key.sanitize();
        assert_eq!(key.fov_deg, 10.0, "fov_deg should clamp to 10° minimum");
    }

    #[test]
    fn sanitize_clamps_fov_deg_above_maximum() {
        let mut key = CameraKey::new(Time(0.0), (0.0, 0.0, 0.0), (1.0, 0.0, 0.0), 200.0);
        key.sanitize();
        assert_eq!(key.fov_deg, 170.0, "fov_deg should clamp to 170° maximum");
    }

    #[test]
    fn sanitize_resolves_degenerate_look_at_to_plus_x() {
        // look_at == pos is degenerate (undefined direction). sanitize sets
        // look_at = pos + (1, 0, 0) (canonical +X forward per §2.8).
        let mut key = CameraKey::new(Time(0.0), (5.0, 3.0, 2.0), (5.0, 3.0, 2.0), 60.0);
        key.sanitize();
        assert_eq!(
            key.look_at,
            (6.0, 3.0, 2.0),
            "degenerate look_at should resolve to pos + (1, 0, 0)"
        );
    }

    #[test]
    fn sanitize_preserves_valid_look_at() {
        // Non-degenerate look_at is preserved unchanged.
        let mut key = CameraKey::new(Time(0.0), (0.0, 0.0, 0.0), (0.0, 0.0, 10.0), 60.0);
        key.sanitize();
        assert_eq!(
            key.look_at,
            (0.0, 0.0, 10.0),
            "valid look_at should be preserved"
        );
    }

    #[test]
    fn sanitize_is_idempotent_on_valid_key() {
        let mut key = CameraKey::new(Time(0.0), (0.0, 0.0, 0.0), (1.0, 0.0, 0.0), 60.0);
        let before = key.clone();
        key.sanitize();
        assert_eq!(
            key, before,
            "sanitize on already-valid key should be a no-op"
        );
    }

    #[test]
    fn sanitize_preserves_pos_and_time() {
        // sanitize affects fov_deg and (conditionally) look_at; pos and t
        // are never modified, even when sanitize triggers (here both
        // out-of-range fov_deg AND degenerate look_at trigger).
        let mut key = CameraKey::new(Time(1.5), (3.0, 4.0, 5.0), (3.0, 4.0, 5.0), 200.0);
        key.sanitize();
        assert_eq!(key.pos, (3.0, 4.0, 5.0), "pos should be preserved");
        assert_eq!(key.t, Time(1.5), "t should be preserved");
        // (look_at and fov_deg are modified per the degenerate + clamp cases)
        assert_eq!(key.look_at, (4.0, 4.0, 5.0));
        assert_eq!(key.fov_deg, 170.0);
    }

    #[test]
    fn test_camera_key_lerp() {
        let key1 = CameraKey::new(Time(0.0), (0.0, 0.0, 0.0), (0.0, 0.0, 0.0), 60.0);
        let key2 = CameraKey::new(Time(2.0), (10.0, 10.0, 10.0), (10.0, 10.0, 10.0), 90.0);

        let mid = key1.lerp(&key2, 0.5);
        assert!((mid.t.0 - 1.0).abs() < 0.0001);
        assert!((mid.pos.0 - 5.0).abs() < 0.0001);
        assert!((mid.fov_deg - 75.0).abs() < 0.0001);
    }

    #[test]
    fn test_camera_key_display() {
        let key = CameraKey::new(Time(1.5), (1.0, 2.0, 3.0), (0.0, 0.0, 0.0), 60.0);
        let s = format!("{}", key);
        assert!(s.contains("1.50s"));
        assert!(s.contains("fov=60"));
    }

    // ============================================================
    // Timeline helper tests
    // ============================================================

    #[test]
    fn test_timeline_empty() {
        let tl = Timeline::empty();
        assert!(tl.is_empty());
        assert!(tl.duration.is_zero());
    }

    #[test]
    fn test_timeline_is_empty() {
        let tl = Timeline::new("test", 10.0);
        assert!(tl.is_empty());

        let mut tl2 = Timeline::new("test2", 10.0);
        tl2.tracks.push(Track::Camera { keyframes: vec![] });
        assert!(!tl2.is_empty());
    }

    #[test]
    fn test_timeline_track_count() {
        let mut tl = Timeline::new("test", 10.0);
        assert_eq!(tl.track_count(), 0);

        tl.tracks.push(Track::Camera { keyframes: vec![] });
        tl.tracks.push(Track::animation(1, "run", Time(0.0)));
        assert_eq!(tl.track_count(), 2);
    }

    #[test]
    fn test_timeline_camera_track_count() {
        let mut tl = Timeline::new("test", 10.0);
        tl.tracks.push(Track::Camera { keyframes: vec![] });
        tl.tracks.push(Track::Camera { keyframes: vec![] });
        tl.tracks.push(Track::animation(1, "run", Time(0.0)));

        assert_eq!(tl.camera_track_count(), 2);
    }

    #[test]
    fn test_timeline_audio_track_count() {
        let mut tl = Timeline::new("test", 10.0);
        tl.tracks.push(Track::audio("music", Time(0.0), 1.0));
        tl.tracks.push(Track::audio("sfx", Time(1.0), 0.5));
        tl.tracks.push(Track::Camera { keyframes: vec![] });

        assert_eq!(tl.audio_track_count(), 2);
    }

    #[test]
    fn test_timeline_animation_track_count() {
        let mut tl = Timeline::new("test", 10.0);
        tl.tracks.push(Track::animation(1, "run", Time(0.0)));
        tl.tracks.push(Track::animation(2, "walk", Time(1.0)));
        tl.tracks.push(Track::audio("music", Time(0.0), 1.0));

        assert_eq!(tl.animation_track_count(), 2);
    }

    #[test]
    fn test_timeline_fx_track_count() {
        let mut tl = Timeline::new("test", 10.0);
        tl.tracks
            .push(Track::fx("explosion", Time(0.0), serde_json::json!({})));
        tl.tracks
            .push(Track::fx("particles", Time(1.0), serde_json::json!({})));
        tl.tracks.push(Track::Camera { keyframes: vec![] });

        assert_eq!(tl.fx_track_count(), 2);
    }

    #[test]
    fn test_timeline_add_track() {
        let mut tl = Timeline::new("test", 10.0);
        tl.add_track(Track::Camera { keyframes: vec![] });
        assert_eq!(tl.track_count(), 1);
    }

    #[test]
    fn test_timeline_add_camera_track() {
        let mut tl = Timeline::new("test", 10.0);
        let keyframes = vec![CameraKey::at_origin(60.0)];
        tl.add_camera_track(keyframes);
        assert_eq!(tl.camera_track_count(), 1);
    }

    #[test]
    fn test_timeline_add_audio_track() {
        let mut tl = Timeline::new("test", 10.0);
        tl.add_audio_track("music", Time(0.0), 0.8);
        assert_eq!(tl.audio_track_count(), 1);
    }

    #[test]
    fn test_timeline_total_keyframes() {
        let mut tl = Timeline::new("test", 10.0);
        tl.tracks.push(Track::Camera {
            keyframes: vec![CameraKey::at_origin(60.0), CameraKey::at_origin(60.0)],
        });
        tl.tracks.push(Track::animation(1, "run", Time(0.0)));
        tl.tracks.push(Track::audio("music", Time(0.0), 1.0));

        // Only camera keyframes are counted by total_keyframes()
        assert_eq!(tl.total_keyframes(), 2);
    }

    #[test]
    fn test_timeline_duration_secs() {
        let tl = Timeline::new("test", 15.5);
        assert!((tl.duration_secs() - 15.5).abs() < 0.0001);
    }

    #[test]
    fn test_timeline_display() {
        let mut tl = Timeline::new("cutscene", 30.0);
        tl.add_track(Track::Camera { keyframes: vec![] });
        let s = format!("{}", tl);
        assert!(s.contains("cutscene"));
        assert!(s.contains("30.00s"));
        assert!(s.contains("1 track"));
    }

    #[test]
    fn test_timeline_default() {
        let tl = Timeline::default();
        assert!(tl.is_empty());
        assert!(tl.duration.is_zero());
    }

    // ============================================================
    // SequencerEvent helper tests
    // ============================================================

    #[test]
    fn test_sequencer_event_type_name() {
        let cam = SequencerEvent::CameraKey(CameraKey::at_origin(60.0));
        assert_eq!(cam.type_name(), "CameraKey");

        let anim = SequencerEvent::AnimStart {
            target: 1,
            clip: "run".into(),
        };
        assert_eq!(anim.type_name(), "AnimStart");

        let audio = SequencerEvent::AudioPlay {
            clip: "music".into(),
            volume: 1.0,
        };
        assert_eq!(audio.type_name(), "AudioPlay");

        let fx = SequencerEvent::FxTrigger {
            name: "explosion".into(),
            params: serde_json::json!({}),
        };
        assert_eq!(fx.type_name(), "FxTrigger");
    }

    #[test]
    fn test_sequencer_event_is_camera_key() {
        let cam = SequencerEvent::CameraKey(CameraKey::at_origin(60.0));
        assert!(cam.is_camera_key());

        let anim = SequencerEvent::AnimStart {
            target: 1,
            clip: "run".into(),
        };
        assert!(!anim.is_camera_key());
    }

    #[test]
    fn test_sequencer_event_is_anim_start() {
        let anim = SequencerEvent::AnimStart {
            target: 1,
            clip: "run".into(),
        };
        assert!(anim.is_anim_start());

        let cam = SequencerEvent::CameraKey(CameraKey::at_origin(60.0));
        assert!(!cam.is_anim_start());
    }

    #[test]
    fn test_sequencer_event_is_audio_play() {
        let audio = SequencerEvent::AudioPlay {
            clip: "music".into(),
            volume: 1.0,
        };
        assert!(audio.is_audio_play());

        let cam = SequencerEvent::CameraKey(CameraKey::at_origin(60.0));
        assert!(!cam.is_audio_play());
    }

    #[test]
    fn test_sequencer_event_is_fx_trigger() {
        let fx = SequencerEvent::FxTrigger {
            name: "explosion".into(),
            params: serde_json::json!({}),
        };
        assert!(fx.is_fx_trigger());

        let cam = SequencerEvent::CameraKey(CameraKey::at_origin(60.0));
        assert!(!cam.is_fx_trigger());
    }

    #[test]
    fn test_sequencer_event_as_camera_key() {
        let key = CameraKey::at_origin(60.0);
        let ev = SequencerEvent::CameraKey(key.clone());

        let retrieved = ev.as_camera_key();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().t, key.t);

        let audio = SequencerEvent::AudioPlay {
            clip: "music".into(),
            volume: 1.0,
        };
        assert!(audio.as_camera_key().is_none());
    }

    #[test]
    fn test_sequencer_event_animation_clip() {
        let anim = SequencerEvent::AnimStart {
            target: 1,
            clip: "run".into(),
        };
        assert_eq!(anim.animation_clip(), Some("run"));

        let cam = SequencerEvent::CameraKey(CameraKey::at_origin(60.0));
        assert!(cam.animation_clip().is_none());
    }

    #[test]
    fn test_sequencer_event_audio_clip() {
        let audio = SequencerEvent::AudioPlay {
            clip: "music.ogg".into(),
            volume: 1.0,
        };
        assert_eq!(audio.audio_clip(), Some("music.ogg"));

        let cam = SequencerEvent::CameraKey(CameraKey::at_origin(60.0));
        assert!(cam.audio_clip().is_none());
    }

    #[test]
    fn test_sequencer_event_fx_name() {
        let fx = SequencerEvent::FxTrigger {
            name: "explosion".into(),
            params: serde_json::json!({}),
        };
        assert_eq!(fx.fx_name(), Some("explosion"));

        let cam = SequencerEvent::CameraKey(CameraKey::at_origin(60.0));
        assert!(cam.fx_name().is_none());
    }

    #[test]
    fn test_sequencer_event_factory_camera_key() {
        let key = CameraKey::at_origin(60.0);
        let ev = SequencerEvent::camera_key(key);
        assert!(ev.is_camera_key());
    }

    #[test]
    fn test_sequencer_event_factory_anim_start() {
        let ev = SequencerEvent::anim_start(42, "dance");
        assert!(ev.is_anim_start());
        assert_eq!(ev.animation_clip(), Some("dance"));
    }

    #[test]
    fn test_sequencer_event_factory_audio_play() {
        let ev = SequencerEvent::audio_play("theme.mp3", 0.8);
        assert!(ev.is_audio_play());
        assert_eq!(ev.audio_clip(), Some("theme.mp3"));
    }

    #[test]
    fn test_sequencer_event_factory_fx_trigger() {
        let ev = SequencerEvent::fx_trigger("particles", serde_json::json!({"count": 100}));
        assert!(ev.is_fx_trigger());
        assert_eq!(ev.fx_name(), Some("particles"));
    }

    #[test]
    fn test_sequencer_event_display_camera_key() {
        let ev = SequencerEvent::CameraKey(CameraKey::at_origin(60.0));
        let s = format!("{}", ev);
        assert!(s.contains("CameraKey"));
    }

    #[test]
    fn test_sequencer_event_display_anim_start() {
        let ev = SequencerEvent::anim_start(10, "walk");
        let s = format!("{}", ev);
        assert!(s.contains("AnimStart"));
        assert!(s.contains("target=10"));
        assert!(s.contains("walk"));
    }

    #[test]
    fn test_sequencer_event_display_audio_play() {
        let ev = SequencerEvent::audio_play("music.ogg", 0.75);
        let s = format!("{}", ev);
        assert!(s.contains("AudioPlay"));
        assert!(s.contains("music.ogg"));
        assert!(s.contains("0.75"));
    }

    #[test]
    fn test_sequencer_event_display_fx_trigger() {
        let ev = SequencerEvent::fx_trigger("explosion", serde_json::json!({}));
        let s = format!("{}", ev);
        assert!(s.contains("FxTrigger"));
        assert!(s.contains("explosion"));
    }

    #[test]
    fn test_time_display_sub_second() {
        // Test the ms branch (time < 1.0)
        let t = Time(0.25);
        let s = format!("{}", t);
        assert!(s.contains("250ms"));
    }

    #[test]
    fn test_track_display_audio() {
        let audio = Track::audio("thunder.wav", Time(1.5), 0.9);
        let s = format!("{}", audio);
        assert!(s.contains("Audio"));
        assert!(s.contains("thunder.wav"));
        assert!(s.contains("0.90"));
    }

    #[test]
    fn test_track_display_fx() {
        let fx = Track::fx("smoke", Time(2.0), serde_json::json!({"density": 0.5}));
        let s = format!("{}", fx);
        assert!(s.contains("Fx"));
        assert!(s.contains("smoke"));
    }
}
