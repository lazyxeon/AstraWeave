//! One locomotion graph. It samples `astraweave_render` clips and does not
//! invent a second skeleton type.

use astraweave_render::animation::{
    AnimationChannel, AnimationClip, ChannelData, Interpolation, Joint, Skeleton, Transform,
};
use glam::Vec3;

use crate::motor::MotorMode;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LocoState {
    Idle,
    Walk,
    Run,
    Sprint,
    CrouchIdle,
    CrouchMove,
    Jump,
    Roll,
    Fly,
    Attack,
    Block,
    Hit,
    Death,
}

/// What the motor and combat are doing this step. The graph only reads this.
#[derive(Clone, Copy, Debug)]
pub struct PoseInput {
    pub mode: MotorMode,
    pub grounded: bool,
    pub crouching: bool,
    pub sprinting: bool,
    pub walking: bool,
    pub moving: bool,
    pub attacking: bool,
    pub blocking: bool,
    pub hit: bool,
    pub dead: bool,
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
    pub fn sync(&mut self, pose: PoseInput, dt: f32) {
        let next = choose_state(pose);
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

pub fn choose_state(pose: PoseInput) -> LocoState {
    if pose.dead {
        LocoState::Death
    } else if pose.attacking {
        LocoState::Attack
    } else if pose.hit {
        LocoState::Hit
    } else if pose.blocking {
        LocoState::Block
    } else if pose.mode == MotorMode::Fly {
        LocoState::Fly
    } else if pose.mode == MotorMode::Roll {
        LocoState::Roll
    } else if !pose.grounded {
        LocoState::Jump
    } else if pose.crouching && pose.moving {
        LocoState::CrouchMove
    } else if pose.crouching {
        LocoState::CrouchIdle
    } else if pose.sprinting && pose.moving {
        LocoState::Sprint
    } else if pose.walking && pose.moving {
        LocoState::Walk
    } else if pose.moving {
        LocoState::Run
    } else {
        LocoState::Idle
    }
}

pub fn clip_name(state: LocoState) -> &'static str {
    match state {
        LocoState::Idle => "Idle_A",
        LocoState::Walk => "Walking_A",
        LocoState::Run => "Running_A",
        LocoState::Sprint => "Running_B",
        LocoState::CrouchIdle => "Crouching",
        LocoState::CrouchMove => "Sneaking",
        LocoState::Jump | LocoState::Fly => "Jump_Idle",
        LocoState::Roll => "Dodge_Forward",
        LocoState::Attack => "Melee_1H_Attack_Slice_Horizontal",
        LocoState::Block => "Melee_Blocking",
        LocoState::Hit => "Hit_A",
        LocoState::Death => "Death_A",
    }
}

/// Seconds into the clip. One-shots take `phase` in 0..1 across the clip.
/// Looping states wrap `graph_time`. Death holds the last frame.
pub fn visual_time(
    state: LocoState,
    graph_time: f32,
    phase: f32,
    clip_duration: f32,
) -> (f32, bool) {
    let duration = clip_duration.max(0.001);
    match state {
        LocoState::Roll | LocoState::Attack | LocoState::Hit => {
            (phase.clamp(0.0, 1.0) * duration, false)
        }
        LocoState::Death => ((graph_time).min(duration - 0.001), false),
        _ => (graph_time.rem_euclid(duration), true),
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
        let pose = PoseInput {
            mode: MotorMode::Roll,
            moving: true,
            grounded: true,
            ..idle_pose()
        };
        graph.sync(pose, 0.016);
        assert_eq!(graph.state, LocoState::Roll);
        graph.sync(pose, 0.016);
        assert!(graph.time > 0.0);
    }

    #[test]
    fn every_action_maps_to_a_distinct_clip() {
        let cases = [
            (LocoState::Idle, "Idle_A"),
            (LocoState::Walk, "Walking_A"),
            (LocoState::Run, "Running_A"),
            (LocoState::Sprint, "Running_B"),
            (LocoState::Jump, "Jump_Idle"),
            (LocoState::Fly, "Jump_Idle"),
            (LocoState::CrouchIdle, "Crouching"),
            (LocoState::CrouchMove, "Sneaking"),
            (LocoState::Roll, "Dodge_Forward"),
            (LocoState::Attack, "Melee_1H_Attack_Slice_Horizontal"),
            (LocoState::Block, "Melee_Blocking"),
            (LocoState::Hit, "Hit_A"),
            (LocoState::Death, "Death_A"),
        ];
        for (state, name) in cases {
            assert_eq!(clip_name(state), name);
        }
        assert_eq!(
            choose_state(PoseInput {
                dead: true,
                attacking: true,
                ..idle_pose()
            }),
            LocoState::Death
        );
        assert_eq!(
            choose_state(PoseInput {
                attacking: true,
                ..idle_pose()
            }),
            LocoState::Attack
        );
        assert_eq!(
            choose_state(PoseInput {
                hit: true,
                blocking: true,
                ..idle_pose()
            }),
            LocoState::Hit
        );
        assert_eq!(
            choose_state(PoseInput {
                blocking: true,
                ..idle_pose()
            }),
            LocoState::Block
        );
        assert_eq!(
            choose_state(PoseInput {
                mode: MotorMode::Fly,
                ..idle_pose()
            }),
            LocoState::Fly
        );
        assert_eq!(
            choose_state(PoseInput {
                walking: true,
                moving: true,
                grounded: true,
                ..idle_pose()
            }),
            LocoState::Walk
        );
        assert_eq!(
            choose_state(PoseInput {
                sprinting: true,
                moving: true,
                grounded: true,
                ..idle_pose()
            }),
            LocoState::Sprint
        );
        assert_eq!(
            choose_state(PoseInput {
                moving: true,
                grounded: true,
                ..idle_pose()
            }),
            LocoState::Run
        );
        assert_eq!(
            choose_state(PoseInput {
                crouching: true,
                moving: true,
                grounded: true,
                ..idle_pose()
            }),
            LocoState::CrouchMove
        );
        assert_eq!(
            choose_state(PoseInput {
                grounded: false,
                ..idle_pose()
            }),
            LocoState::Jump
        );
        let (death_time, death_loops) = visual_time(LocoState::Death, 9.0, 0.0, 2.375);
        assert!(!death_loops);
        assert!(death_time < 2.375);
        let (swing, swing_loops) = visual_time(LocoState::Attack, 0.0, 0.5, 1.5);
        assert!(!swing_loops);
        assert!((swing - 0.75).abs() < 1e-4);
        let (idle, idle_loops) = visual_time(LocoState::Idle, 2.6, 0.0, 2.5);
        assert!(idle_loops);
        assert!(idle < 2.5);
    }

    fn idle_pose() -> PoseInput {
        PoseInput {
            mode: MotorMode::Locomotion,
            grounded: true,
            crouching: false,
            sprinting: false,
            walking: false,
            moving: false,
            attacking: false,
            blocking: false,
            hit: false,
            dead: false,
        }
    }
}
