//! Animation bridge: connects the editor's AnimationPanel UI to runtime animation playback.
//!
//! Manages per-entity animation state, processes `AnimationAction` commands,
//! and ticks clip playback each frame to produce transforms for skinned entities.

use crate::panels::animation_panel::AnimationAction;
use crate::viewport::entity_renderer::{
    GltfAnimationClip, GltfChannelProperty, GltfInterpolation, GltfSkeleton,
};
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
    /// Per-entity skeleton data (from GLTF extraction).
    entity_skeletons: HashMap<u64, GltfSkeleton>,
    /// Per-entity animation clips (from GLTF extraction).
    entity_anim_clips: HashMap<u64, Vec<GltfAnimationClip>>,
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
            entity_skeletons: HashMap::new(),
            entity_anim_clips: HashMap::new(),
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
        self.entity_skeletons.remove(&entity_id);
        self.entity_anim_clips.remove(&entity_id);
    }

    // ========================================================================
    // Phase 4: Skeleton storage & skinning
    // ========================================================================

    /// Store a skeleton for an entity (extracted from its GLTF mesh).
    pub fn set_entity_skeleton(&mut self, entity_id: u64, skeleton: GltfSkeleton) {
        self.entity_skeletons.insert(entity_id, skeleton);
    }

    /// Store animation clips for an entity.
    pub fn set_entity_clips(&mut self, entity_id: u64, clips: Vec<GltfAnimationClip>) {
        // Also register clips in the bridge library
        for clip in &clips {
            let exists = self.clips.iter().any(|c| c.name == clip.name);
            if !exists {
                self.register_clip(&clip.name, clip.duration, clip.channels.len());
            }
        }
        self.entity_anim_clips.insert(entity_id, clips);
    }

    /// Check if an entity has a skeleton.
    pub fn has_skeleton(&self, entity_id: u64) -> bool {
        self.entity_skeletons.contains_key(&entity_id)
    }

    /// Sample an animation clip at a given time, producing per-joint local transforms.
    fn sample_clip(
        clip: &GltfAnimationClip,
        skeleton: &GltfSkeleton,
        time: f32,
    ) -> Vec<JointTransform> {
        let mut transforms: Vec<JointTransform> = skeleton
            .joints
            .iter()
            .map(|j| {
                // Decompose rest-pose local transform
                let (_, rot, _) = j.local_transform.to_scale_rotation_translation();
                let trans = j.local_transform.col(3).truncate();
                let scale_x = j.local_transform.col(0).truncate().length();
                let scale_y = j.local_transform.col(1).truncate().length();
                let scale_z = j.local_transform.col(2).truncate().length();
                JointTransform {
                    translation: trans,
                    rotation: rot,
                    scale: Vec3::new(scale_x, scale_y, scale_z),
                }
            })
            .collect();

        // Apply animation channels
        for channel in &clip.channels {
            if channel.times.is_empty() || channel.values.is_empty() {
                continue;
            }
            if channel.joint_index >= transforms.len() {
                continue;
            }

            let t = time.clamp(0.0, clip.duration);
            let value = Self::interpolate_channel(channel, t);

            match channel.property {
                GltfChannelProperty::Translation => {
                    if value.len() >= 3 {
                        transforms[channel.joint_index].translation =
                            Vec3::new(value[0], value[1], value[2]);
                    }
                }
                GltfChannelProperty::Rotation => {
                    if value.len() >= 4 {
                        transforms[channel.joint_index].rotation =
                            Quat::from_xyzw(value[0], value[1], value[2], value[3]).normalize();
                    }
                }
                GltfChannelProperty::Scale => {
                    if value.len() >= 3 {
                        transforms[channel.joint_index].scale =
                            Vec3::new(value[0], value[1], value[2]);
                    }
                }
            }
        }

        transforms
    }

    /// Interpolate a channel at a given time, returning the interpolated value.
    fn interpolate_channel(
        channel: &crate::viewport::entity_renderer::GltfAnimChannel,
        time: f32,
    ) -> Vec<f32> {
        let times = &channel.times;
        let values = &channel.values;

        if times.len() <= 1 || values.is_empty() {
            return values.first().cloned().unwrap_or_default();
        }

        // Find the two keyframes surrounding `time`
        let mut i = 0;
        while i < times.len() - 1 && times[i + 1] < time {
            i += 1;
        }

        if i >= times.len() - 1 {
            return values.last().cloned().unwrap_or_default();
        }

        let t0 = times[i];
        let t1 = times[i + 1];
        let alpha = if (t1 - t0).abs() > f32::EPSILON {
            ((time - t0) / (t1 - t0)).clamp(0.0, 1.0)
        } else {
            0.0
        };

        let v0 = &values[i];
        let v1 = &values.get(i + 1).unwrap_or(v0);

        match channel.interpolation {
            GltfInterpolation::Step => v0.clone(),
            GltfInterpolation::Linear | GltfInterpolation::CubicSpline => {
                // For quaternions (4 components), use slerp
                if v0.len() == 4 {
                    let q0 = Quat::from_xyzw(v0[0], v0[1], v0[2], v0[3]).normalize();
                    let q1 = Quat::from_xyzw(v1[0], v1[1], v1[2], v1[3]).normalize();
                    let q = q0.slerp(q1, alpha);
                    vec![q.x, q.y, q.z, q.w]
                } else {
                    // Linear interpolation for translation/scale
                    v0.iter()
                        .zip(v1.iter())
                        .map(|(&a, &b)| a + (b - a) * alpha)
                        .collect()
                }
            }
        }
    }

    /// Compute joint matrices for an entity based on its current animation state.
    /// Returns `None` if the entity has no skeleton or no active animation.
    pub fn compute_joint_matrices(&self, entity_id: u64) -> Option<Vec<Mat4>> {
        let skeleton = self.entity_skeletons.get(&entity_id)?;
        let state = self.entity_states.get(&entity_id)?;
        let clip_index = state.clip_index?;
        let clips = self.entity_anim_clips.get(&entity_id)?;
        let clip = clips.get(clip_index)?;

        let local_transforms = Self::sample_clip(clip, skeleton, state.time);

        // Build world-space joint matrices: parent_world * local * inverse_bind
        let mut world_matrices = vec![Mat4::IDENTITY; skeleton.joints.len()];
        let mut joint_matrices = vec![Mat4::IDENTITY; skeleton.joints.len()];

        for ji in 0..skeleton.joints.len() {
            let jt = &local_transforms[ji];
            let local =
                Mat4::from_scale_rotation_translation(jt.scale, jt.rotation, jt.translation);

            let world = if let Some(pi) = skeleton.joints[ji].parent_index {
                world_matrices[pi] * local
            } else {
                local
            };

            world_matrices[ji] = world;
            joint_matrices[ji] = world * skeleton.joints[ji].inverse_bind_matrix;
        }

        Some(joint_matrices)
    }
}
