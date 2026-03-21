//! Animation bridge: connects the editor's AnimationPanel UI to runtime animation playback.
//!
//! Manages per-entity animation state, processes `AnimationAction` commands,
//! and ticks clip playback each frame to produce transforms for skinned entities.

use crate::panels::animation_panel::AnimationAction;
use glam::{Mat4, Quat, Vec3};
use std::collections::HashMap;

/// Per-entity animation playback state.
#[derive(Clone, Debug)]
pub struct EntityAnimationState {
    /// Index of the currently playing clip (into the clip library).
    pub clip_index: Option<usize>,
    /// Current playback time in seconds.
    pub time: f32,
    /// Playback speed multiplier.
    pub speed: f32,
    /// Whether the clip loops.
    pub looping: bool,
    /// Whether playback is active.
    pub playing: bool,
}

impl Default for EntityAnimationState {
    fn default() -> Self {
        Self {
            clip_index: None,
            time: 0.0,
            speed: 1.0,
            looping: true,
            playing: false,
        }
    }
}

/// A lightweight clip definition stored in the bridge.
#[derive(Clone, Debug)]
pub struct ClipEntry {
    pub id: u32,
    pub name: String,
    pub duration: f32,
    /// Number of channels (joints) in this clip.
    pub channel_count: usize,
}

/// Computed animation output for a single entity (one frame).
#[derive(Clone, Debug)]
pub struct AnimationOutput {
    /// Local-space joint transforms (translation, rotation, scale per joint).
    pub joint_transforms: Vec<JointTransform>,
    /// World-space joint matrices (with inverse bind applied), ready for skinning.
    pub joint_matrices: Vec<Mat4>,
}

/// A single joint's local transform.
#[derive(Clone, Debug)]
pub struct JointTransform {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl Default for JointTransform {
    fn default() -> Self {
        Self {
            translation: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        }
    }
}

/// Animation bridge that owns the clip library and per-entity playback state.
pub struct EditorAnimationBridge {
    /// Global clip library (shared across entities).
    clips: Vec<ClipEntry>,
    /// Per-entity animation state, keyed by entity ID.
    entity_states: HashMap<u64, EntityAnimationState>,
    /// The currently "global" playback state used by the animation panel preview.
    preview_state: EntityAnimationState,
    /// Time spent in last tick (ms), for profiling.
    pub last_tick_ms: f32,
    next_clip_id: u32,
}

impl Default for EditorAnimationBridge {
    fn default() -> Self {
        Self::new()
    }
}

impl EditorAnimationBridge {
    pub fn new() -> Self {
        Self {
            clips: Vec::new(),
            entity_states: HashMap::new(),
            preview_state: EntityAnimationState::default(),
            last_tick_ms: 0.0,
            next_clip_id: 1,
        }
    }

    /// Register a clip in the library. Returns the assigned clip ID.
    pub fn register_clip(&mut self, name: &str, duration: f32, channel_count: usize) -> u32 {
        let id = self.next_clip_id;
        self.next_clip_id += 1;
        self.clips.push(ClipEntry {
            id,
            name: name.to_string(),
            duration,
            channel_count,
        });
        id
    }

    /// Get all registered clips.
    pub fn clips(&self) -> &[ClipEntry] {
        &self.clips
    }

    /// Get the animation state for a specific entity.
    pub fn entity_state(&self, entity_id: u64) -> Option<&EntityAnimationState> {
        self.entity_states.get(&entity_id)
    }

    /// Assign a clip to an entity and start playing.
    pub fn assign_clip(&mut self, entity_id: u64, clip_index: usize) {
        let state = self
            .entity_states
            .entry(entity_id)
            .or_insert_with(EntityAnimationState::default);
        state.clip_index = Some(clip_index);
        state.time = 0.0;
        state.playing = true;
    }

    /// Stop animation on an entity.
    pub fn stop_entity(&mut self, entity_id: u64) {
        if let Some(state) = self.entity_states.get_mut(&entity_id) {
            state.playing = false;
            state.time = 0.0;
        }
    }

    /// Process animation actions from the panel.
    pub fn process_actions(&mut self, actions: Vec<AnimationAction>) {
        for action in actions {
            match action {
                AnimationAction::PlayClip { clip_id } => {
                    // Find clip index by ID
                    if let Some(idx) = self.clips.iter().position(|c| c.id == clip_id) {
                        self.preview_state.clip_index = Some(idx);
                        self.preview_state.playing = true;
                        self.preview_state.time = 0.0;
                    }
                }
                AnimationAction::PauseClip => {
                    self.preview_state.playing = false;
                }
                AnimationAction::StopClip => {
                    self.preview_state.playing = false;
                    self.preview_state.time = 0.0;
                }
                AnimationAction::SetSpeed { speed } => {
                    self.preview_state.speed = speed;
                }
                AnimationAction::SeekTo { time } => {
                    self.preview_state.time = time;
                }
                AnimationAction::CreateClip { name } => {
                    self.register_clip(&name, 1.0, 0);
                }
                AnimationAction::DeleteClip { clip_id } => {
                    self.clips.retain(|c| c.id != clip_id);
                }
                AnimationAction::SetLoopMode {
                    clip_id: _,
                    loop_mode: _,
                } => {
                    // Loop mode is tracked in the panel; bridge respects per-entity looping flag
                }
                AnimationAction::CreateController { .. }
                | AnimationAction::TriggerTransition { .. }
                | AnimationAction::SaveClip { .. }
                | AnimationAction::AddKeyframe { .. }
                | AnimationAction::DeleteKeyframe { .. } => {
                    // These are editing operations handled by the panel itself
                }
            }
        }
    }

    /// Advance all active animations by `dt` seconds.
    /// Returns the set of entity IDs whose animations changed this frame.
    pub fn tick(&mut self, dt: f32) -> Vec<u64> {
        let start = std::time::Instant::now();
        let mut changed = Vec::new();

        // Tick preview state
        if self.preview_state.playing {
            if let Some(idx) = self.preview_state.clip_index {
                if let Some(clip) = self.clips.get(idx) {
                    self.preview_state.time += dt * self.preview_state.speed;
                    if self.preview_state.time >= clip.duration {
                        if self.preview_state.looping {
                            self.preview_state.time %= clip.duration;
                        } else {
                            self.preview_state.time = clip.duration;
                            self.preview_state.playing = false;
                        }
                    }
                }
            }
        }

        // Tick per-entity states
        let clip_durations: Vec<f32> = self.clips.iter().map(|c| c.duration).collect();
        for (&entity_id, state) in self.entity_states.iter_mut() {
            if !state.playing {
                continue;
            }
            if let Some(idx) = state.clip_index {
                if let Some(&duration) = clip_durations.get(idx) {
                    state.time += dt * state.speed;
                    if state.time >= duration {
                        if state.looping {
                            state.time %= duration;
                        } else {
                            state.time = duration;
                            state.playing = false;
                        }
                    }
                    changed.push(entity_id);
                }
            }
        }

        self.last_tick_ms = start.elapsed().as_secs_f32() * 1000.0;
        changed
    }

    /// Get the preview playback time (for the animation panel timeline cursor).
    pub fn preview_time(&self) -> f32 {
        self.preview_state.time
    }

    /// Returns true if preview is actively playing.
    pub fn is_preview_playing(&self) -> bool {
        self.preview_state.playing
    }

    /// Get the current preview clip index.
    pub fn preview_clip_index(&self) -> Option<usize> {
        self.preview_state.clip_index
    }

    /// Returns the number of entities with active animations.
    pub fn active_animation_count(&self) -> usize {
        self.entity_states.values().filter(|s| s.playing).count()
    }

    /// Remove animation state for a deleted entity.
    pub fn remove_entity(&mut self, entity_id: u64) {
        self.entity_states.remove(&entity_id);
    }
}
