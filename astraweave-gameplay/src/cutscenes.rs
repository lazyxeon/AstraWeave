use astraweave_cinematics::{CameraKey, Time};
use glam::Vec3;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[non_exhaustive]
pub enum Cue {
    /// Move the camera to a specific world position with a look-at target.
    ///
    /// Post-C.7.A (Unified Camera campaign), `CameraTo` uses look_at
    /// storage instead of the pre-C.7.A yaw/pitch storage. The semantic
    /// shift aligns with [`astraweave_cinematics::CameraKey`]'s canonical
    /// look_at convention; `CutsceneState::tick` emits a `CameraKey`
    /// (wrapped in [`CutsceneTickEvent::Camera`]) constructed from this
    /// variant's fields via the gameplay→cinematics boundary conversion.
    ///
    /// Fields:
    /// - `pos`: camera world position (preserved as `Vec3` for gameplay-
    ///   layer ergonomics; converted to `(f32, f32, f32)` tuple at the
    ///   boundary).
    /// - `look_at`: world-space target the camera looks at.
    /// - `fov_deg`: vertical field of view in degrees (matches
    ///   `CameraKey.fov_deg`'s unit/name discipline).
    /// - `time`: cue duration in seconds (semantically distinct from
    ///   `CameraKey.t: Time` which is absolute timestamp; gameplay's
    ///   Timeline carries cue-duration semantics, cinematics's Timeline
    ///   carries timestamp semantics — see `Timeline` doc comment).
    CameraTo {
        pos: Vec3,
        look_at: Vec3,
        fov_deg: f32,
        time: f32,
    },
    Title {
        text: String,
        time: f32,
    },
    Wait {
        time: f32,
    },
}

/// Structured event emitted by [`CutsceneState::tick`] for each frame.
///
/// Replaces the pre-C.7.A triple-Optional tuple
/// `(Option<(Vec3, f32, f32)>, Option<String>, bool)` introduced in
/// early cinematics work. Added in Unified Camera campaign sub-phase
/// C.7.A per planning round Q3's structured-enum decision (vs preserving
/// tuple shape with swapped inner type).
///
/// Variants:
///
/// - [`CutsceneTickEvent::Camera`]: a camera cue is active this frame;
///   the embedded [`CameraKey`] is the canonical cinematics keyframe
///   shape. Consumers pass this to
///   [`astraweave_render::Renderer::tick_cinematics`] or
///   `apply_camera_key` (C.7.B's rewrite of `cutscene_render_demo` is
///   the canonical reference for consumption pattern; the C.7.A bridge
///   inlines an equivalent conversion locally).
/// - [`CutsceneTickEvent::Title`]: a title cue is active; the embedded
///   `String` is the title text to display.
/// - [`CutsceneTickEvent::Continue`]: no cue is active this frame, but
///   the timeline hasn't finished. Caller should continue ticking
///   (typically by running its default camera controller).
/// - [`CutsceneTickEvent::Done`]: the timeline has finished. Caller can
///   stop ticking.
///
/// Multiple cue types may fire simultaneously in future cinematics work;
/// C.7.A's `CutsceneState::tick` emits at most one event per call (the
/// active cue at the current time index). If multi-event-per-tick
/// becomes needed, the return type can evolve to
/// `Vec<CutsceneTickEvent>` without breaking the enum's variant set.
#[derive(Clone, Debug, PartialEq)]
pub enum CutsceneTickEvent {
    Camera(CameraKey),
    Title(String),
    Continue,
    Done,
}

/// Gameplay-cutscene timeline: a sequence of [`Cue`]s consumed by
/// [`CutsceneState::tick`].
///
/// **Distinct from [`astraweave_cinematics::Timeline`]** (the canonical
/// cinematics data type). `astraweave-gameplay`'s `Timeline` operates
/// on **cue-duration semantics**; `astraweave-cinematics`'s `Timeline`
/// operates on **absolute-timestamp semantics**. Conversion between
/// them happens at the gameplay→cinematics boundary inside
/// [`CutsceneState::tick`], which emits canonical
/// [`CameraKey`](astraweave_cinematics::CameraKey) events via the
/// [`CutsceneTickEvent::Camera`] variant.
///
/// The dual-Timeline pattern reflects the two systems' distinct
/// concerns: gameplay's `Timeline` carries cue-script-level data
/// (a sequence of intentions); cinematics's `Timeline` carries
/// keyframe-data-level data (a sequence of states). They coexist
/// rather than consolidate; C.7.A established the boundary conversion.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Timeline {
    pub cues: Vec<Cue>,
}

pub struct CutsceneState {
    pub idx: usize,
    pub t: f32,
}

impl Default for CutsceneState {
    fn default() -> Self {
        Self::new()
    }
}

impl CutsceneState {
    pub fn new() -> Self {
        Self { idx: 0, t: 0.0 }
    }

    /// Advances the cutscene timeline by `dt` seconds and emits a
    /// structured event describing what's active this frame.
    ///
    /// Post-C.7.A (Unified Camera campaign), the return type is
    /// [`CutsceneTickEvent`] (the pre-C.7.A triple-Optional tuple is
    /// retired). The time-advancement and cue-indexing logic is
    /// preserved exactly; only the emit shape changes.
    ///
    /// The gameplay→cinematics boundary conversion
    /// (`Vec3 → (f32, f32, f32)`) happens inside this function (per
    /// C.7.0 audit L.7.3's resolution); callers receive canonical
    /// `CameraKey`-shaped data without performing tuple conversions
    /// themselves.
    pub fn tick(&mut self, dt: f32, tl: &Timeline) -> CutsceneTickEvent {
        if self.idx >= tl.cues.len() {
            return CutsceneTickEvent::Done;
        }
        self.t += dt;
        match &tl.cues[self.idx] {
            Cue::CameraTo {
                pos,
                look_at,
                fov_deg,
                time,
            } => {
                // Boundary conversion: gameplay Vec3 → cinematics tuple.
                // The Time field carries the current cue-elapsed time;
                // this is the snapshot timestamp at this tick. C.7.B's
                // canonical consumer (`Renderer::tick_cinematics` via a
                // proper Timeline) will use absolute-timestamp semantics
                // instead; the bridge in `cutscene_render_demo` applies
                // this key directly to the camera regardless of `t`.
                let key = CameraKey {
                    t: Time(self.t),
                    pos: (pos.x, pos.y, pos.z),
                    look_at: (look_at.x, look_at.y, look_at.z),
                    fov_deg: *fov_deg,
                };
                let cue_done = self.t >= *time;
                if cue_done {
                    self.idx += 1;
                    self.t = 0.0;
                }
                CutsceneTickEvent::Camera(key)
            }
            Cue::Title { text, time } => {
                let text = text.clone();
                let done = self.t >= *time;
                if done {
                    self.idx += 1;
                    self.t = 0.0;
                }
                CutsceneTickEvent::Title(text)
            }
            Cue::Wait { time } => {
                let done = self.t >= *time;
                if done {
                    self.idx += 1;
                    self.t = 0.0;
                }
                CutsceneTickEvent::Continue
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // C.7.A fixture helpers — pre-C.7.A tests used arbitrary yaw/pitch
    // values (e.g., `yaw: 45.0, pitch: -10.0` in raw float units that
    // weren't actual radian camera math; the tests verified tick
    // mechanics, not camera semantics). Post-C.7.A, the equivalent
    // semantic is "camera at `pos` looking toward some target." We use
    // `pos + Vec3::X` (the canonical +X-forward direction per
    // CAMERA_CONVENTIONS.md §2.8) as the default look_at, and a standard
    // `60.0` fov_deg matching CameraKey conventions. Distinct fixtures
    // can vary look_at per their narrative intent.

    #[test]
    fn test_cutscene_state_default() {
        let state = CutsceneState::new();
        assert_eq!(state.idx, 0);
        assert_eq!(state.t, 0.0);
    }

    #[test]
    fn test_camera_to_cue_during_transition() {
        let mut state = CutsceneState::new();
        let pos = Vec3::new(1.0, 2.0, 3.0);
        let look_at = pos + Vec3::X; // +X-forward equivalent
        let timeline = Timeline {
            cues: vec![Cue::CameraTo {
                pos,
                look_at,
                fov_deg: 60.0,
                time: 2.0,
            }],
        };

        // Tick before completion
        let event = state.tick(0.5, &timeline);
        match event {
            CutsceneTickEvent::Camera(key) => {
                assert_eq!(key.pos, (1.0, 2.0, 3.0));
                assert_eq!(key.look_at, (look_at.x, look_at.y, look_at.z));
                assert_eq!(key.fov_deg, 60.0);
                // Cue not yet complete; idx should remain 0 and t should advance.
                assert_eq!(state.idx, 0);
                assert_eq!(state.t, 0.5);
            }
            other => panic!("Expected Camera event, got {:?}", other),
        }
    }

    #[test]
    fn test_camera_to_cue_completes() {
        let mut state = CutsceneState::new();
        let pos = Vec3::new(5.0, 6.0, 7.0);
        let look_at = pos + Vec3::X;
        let timeline = Timeline {
            cues: vec![Cue::CameraTo {
                pos,
                look_at,
                fov_deg: 60.0,
                time: 1.0,
            }],
        };

        // Tick to completion (dt > time)
        let event = state.tick(1.5, &timeline);
        match event {
            CutsceneTickEvent::Camera(_) => {
                // CameraTo emits its key on the completion tick too, then
                // advances; the next tick would return Done.
                assert_eq!(state.idx, 1);
                assert_eq!(state.t, 0.0);
            }
            other => panic!("Expected Camera event on completion tick, got {:?}", other),
        }
    }

    #[test]
    fn test_title_cue_displays_text() {
        let mut state = CutsceneState::new();
        let timeline = Timeline {
            cues: vec![Cue::Title {
                text: "Chapter 1".to_string(),
                time: 3.0,
            }],
        };

        // Tick before completion
        let event = state.tick(1.0, &timeline);
        match event {
            CutsceneTickEvent::Title(text) => {
                assert_eq!(text, "Chapter 1");
                assert_eq!(state.idx, 0);
            }
            other => panic!("Expected Title event, got {:?}", other),
        }

        // Tick to completion
        let event = state.tick(2.5, &timeline);
        match event {
            CutsceneTickEvent::Title(text) => {
                assert_eq!(text, "Chapter 1");
                assert_eq!(state.idx, 1);
            }
            other => panic!("Expected Title event on completion tick, got {:?}", other),
        }
    }

    #[test]
    fn test_wait_cue_progression() {
        let mut state = CutsceneState::new();
        let timeline = Timeline {
            cues: vec![Cue::Wait { time: 2.0 }],
        };

        // Tick before completion
        let event = state.tick(1.0, &timeline);
        assert_eq!(event, CutsceneTickEvent::Continue);

        // Tick to completion — Wait emits Continue on the completion tick,
        // then advances; the next tick would return Done.
        let event = state.tick(1.5, &timeline);
        assert_eq!(event, CutsceneTickEvent::Continue);
        assert_eq!(state.idx, 1);
    }

    #[test]
    fn test_multiple_cues_progression() {
        let mut state = CutsceneState::new();
        let pos = Vec3::ZERO;
        let look_at = pos + Vec3::X;
        let timeline = Timeline {
            cues: vec![
                Cue::Title {
                    text: "Intro".to_string(),
                    time: 1.0,
                },
                Cue::Wait { time: 0.5 },
                Cue::CameraTo {
                    pos,
                    look_at,
                    fov_deg: 60.0,
                    time: 1.0,
                },
            ],
        };

        // Complete first cue (Title)
        let event = state.tick(1.5, &timeline);
        match event {
            CutsceneTickEvent::Title(text) => assert_eq!(text, "Intro"),
            other => panic!("Expected Title event, got {:?}", other),
        }
        assert_eq!(state.idx, 1);

        // Complete second cue (Wait)
        let event = state.tick(0.6, &timeline);
        assert_eq!(event, CutsceneTickEvent::Continue);
        assert_eq!(state.idx, 2);

        // Complete third cue (CameraTo) — emits Camera on the completion
        // tick, then advances. idx becomes 3 (one past the end).
        let event = state.tick(1.1, &timeline);
        match event {
            CutsceneTickEvent::Camera(_) => {}
            other => panic!("Expected Camera event, got {:?}", other),
        }
        assert_eq!(state.idx, 3);
    }

    #[test]
    fn test_empty_timeline_returns_done() {
        let mut state = CutsceneState::new();
        let timeline = Timeline { cues: vec![] };

        let event = state.tick(1.0, &timeline);
        assert_eq!(event, CutsceneTickEvent::Done);
    }

    #[test]
    fn test_tick_after_completion_stays_done() {
        let mut state = CutsceneState::new();
        let timeline = Timeline {
            cues: vec![Cue::Wait { time: 1.0 }],
        };

        // Complete the timeline (one Wait cue with time=1.0; tick(2.0)
        // advances past it).
        state.tick(2.0, &timeline);
        assert_eq!(state.idx, 1);

        // Tick again after completion
        let event = state.tick(1.0, &timeline);
        assert_eq!(event, CutsceneTickEvent::Done);
        assert_eq!(state.idx, 1); // Stays at end
    }

    #[test]
    fn test_state_timer_resets_between_cues() {
        let mut state = CutsceneState::new();
        let timeline = Timeline {
            cues: vec![Cue::Wait { time: 1.0 }, Cue::Wait { time: 1.0 }],
        };

        // Complete first cue
        state.tick(1.5, &timeline);
        assert_eq!(state.idx, 1);
        assert_eq!(state.t, 0.0); // Timer reset

        // Verify timer accumulates for second cue
        state.tick(0.5, &timeline);
        assert_eq!(state.t, 0.5);
    }
}
