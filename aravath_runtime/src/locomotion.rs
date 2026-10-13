//! One locomotion graph. It samples `astraweave_render` clips and does not
//! invent a second skeleton type.

use astraweave_render::animation::{
    AnimationChannel, AnimationClip, ChannelData, Interpolation, Joint, Skeleton, Transform,
};
use glam::Vec3;

use crate::motor::{Motor, MotorMode};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LocoState {
    Idle,
    Walk,
    Run,
    Sprint,
    Crouch,
    Jump,
    Roll,
    Fly,
}

pub struct LocomotionGraph {
    pub state: LocoState,
    pub time: f32,
}

impl Default for LocomotionGraph {
    fn default() -> Self {
        Self {
            state: LocoState::Idle,
            time: 0.0,
        }
    }
}

impl LocomotionGraph {
    pub fn sync(&mut self, motor: &Motor, moving: bool, dt: f32) {
        let next = match motor.mode {
            MotorMode::Fly => LocoState::Fly,
            MotorMode::Roll => LocoState::Roll,
            _ if motor.jumps_used > 0 && !moving => LocoState::Jump,
            _ if motor.crouching => LocoState::Crouch,
            _ if motor.sprinting && moving => LocoState::Sprint,
            _ if moving => LocoState::Run,
            _ => LocoState::Idle,
        };
        if next != self.state {
            self.state = next;
            self.time = 0.0;
        } else {
            self.time += dt;
        }
    }

    /// Sample a clip. Time is wrapped into the clip duration so a looping
    /// idle stays defined. The sample at `duration` matches the last key.
    pub fn sample_clip(clip: &AnimationClip, skeleton: &Skeleton, time: f32) -> Vec<Transform> {
        let t = if clip.duration > 0.0 {
            time.clamp(0.0, clip.duration)
        } else {
            0.0
        };
        clip.sample(t, skeleton)
    }
}

/// A one-joint clip used when no authored glTF is loaded.
pub fn stand_in_clip() -> (Skeleton, AnimationClip) {
    let skeleton = Skeleton {
        joints: vec![Joint {
            name: "root".into(),
            parent_index: None,
            inverse_bind_matrix: glam::Mat4::IDENTITY,
            local_transform: Transform::default(),
        }],
        root_indices: vec![0],
    };
    let clip = AnimationClip {
        name: "stand-in".into(),
        duration: 1.0,
        channels: vec![AnimationChannel {
            target_joint_index: 0,
            times: vec![0.0, 1.0],
            data: ChannelData::Translation(vec![Vec3::ZERO, Vec3::new(0.0, 0.25, 0.0)]),
            interpolation: Interpolation::Linear,
        }],
    };
    (skeleton, clip)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clip_sample_is_stable_at_ends() {
        let (skeleton, clip) = stand_in_clip();
        let start = LocomotionGraph::sample_clip(&clip, &skeleton, 0.0);
        let end = LocomotionGraph::sample_clip(&clip, &skeleton, clip.duration);
        assert_eq!(start.len(), 1);
        assert!(start[0].translation.length() < 1e-4);
        assert!((end[0].translation.y - 0.25).abs() < 1e-3);
        let again = LocomotionGraph::sample_clip(&clip, &skeleton, 0.0);
        assert!((again[0].translation - start[0].translation).length() < 1e-6);
    }

    #[test]
    fn graph_enters_roll_from_the_motor() {
        let mut graph = LocomotionGraph::default();
        let motor = Motor {
            mode: MotorMode::Roll,
            ..Motor::default()
        };
        graph.sync(&motor, true, 0.016);
        assert_eq!(graph.state, LocoState::Roll);
        graph.sync(&motor, true, 0.016);
        assert!(graph.time > 0.0);
        graph.state = LocoState::Walk;
        assert_eq!(graph.state, LocoState::Walk);
    }
}
