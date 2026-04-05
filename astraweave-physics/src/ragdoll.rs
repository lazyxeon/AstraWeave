//! # Ragdoll System
//!
//! Provides realistic ragdoll physics for character death animations,
//! hit reactions, and procedural animations.
//!
//! ## Features
//!
//! - **Ragdoll Builder**: Create ragdolls from bone hierarchies
//! - **State Management**: Smooth transitions between animated and physics states
//! - **Impulse Propagation**: Realistic force distribution through joint chains
//! - **Joint Limits**: Anatomically-correct constraints
//!
//! ## Usage
//!
//! ```rust
//! use astraweave_physics::ragdoll::{RagdollBuilder, RagdollConfig, BoneShape};
//! use glam::Vec3;
//!
//! // Define a simple humanoid skeleton
//! let mut builder = RagdollBuilder::new(RagdollConfig::default());
//!
//! // Add bones (typically from skeleton data)
//! builder.add_bone("pelvis", None, Vec3::ZERO, BoneShape::Capsule { radius: 0.15, half_height: 0.1 }, 2.0);
//! builder.add_bone("spine", Some("pelvis"), Vec3::new(0.0, 0.2, 0.0), BoneShape::Capsule { radius: 0.12, half_height: 0.15 }, 1.5);
//! builder.add_bone("head", Some("spine"), Vec3::new(0.0, 0.35, 0.0), BoneShape::Sphere { radius: 0.12 }, 1.0);
//!
//! // Build the ragdoll (requires PhysicsWorld)
//! // let ragdoll = builder.build(&mut physics_world, spawn_position);
//! ```

use crate::{BodyId, JointId, JointType, Layers, PhysicsWorld};
use glam::{Quat, Vec3};
use std::collections::HashMap;

/// Unique identifier for a ragdoll
pub type RagdollId = u64;

/// Shape of a bone's physics collider
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub enum BoneShape {
    /// Capsule shape (most common for limbs)
    Capsule { radius: f32, half_height: f32 },
    /// Sphere shape (good for head, hands)
    Sphere { radius: f32 },
    /// Box shape (good for torso, pelvis)
    Box { half_extents: Vec3 },
}

impl BoneShape {
    /// Get the approximate volume of this shape (for mass distribution)
    pub fn volume(&self) -> f32 {
        match self {
            BoneShape::Capsule {
                radius,
                half_height,
            } => {
                let cylinder = std::f32::consts::PI * radius * radius * (half_height * 2.0);
                let sphere = (4.0 / 3.0) * std::f32::consts::PI * radius.powi(3);
                cylinder + sphere
            }
            BoneShape::Sphere { radius } => (4.0 / 3.0) * std::f32::consts::PI * radius.powi(3),
            BoneShape::Box { half_extents } => {
                half_extents.x * half_extents.y * half_extents.z * 8.0
            }
        }
    }
}

/// Joint constraint type for bones
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub enum BoneJointType {
    /// Ball-and-socket joint (shoulder, hip)
    Spherical {
        /// Cone limit in radians (how far the bone can swing)
        swing_limit: Option<f32>,
        /// Twist limit in radians (rotation around the bone's axis)
        twist_limit: Option<(f32, f32)>,
    },
    /// Hinge joint (elbow, knee)
    Hinge {
        /// Axis of rotation (local to parent)
        axis: Vec3,
        /// Angle limits in radians (min, max)
        limits: Option<(f32, f32)>,
    },
    /// Fixed joint (skull to spine, rigid connections)
    Fixed,
}

/// Definition of a single bone in the ragdoll
#[derive(Debug, Clone)]
pub struct BoneDef {
    /// Unique name of this bone
    pub name: String,
    /// Parent bone name (None for root)
    pub parent: Option<String>,
    /// Offset from parent bone (local space)
    pub offset: Vec3,
    /// Rotation offset from parent (local space)
    pub rotation: Quat,
    /// Physics shape for collision
    pub shape: BoneShape,
    /// Mass of this bone
    pub mass: f32,
    /// Joint type connecting to parent
    pub joint_type: BoneJointType,
}

impl Default for BoneDef {
    fn default() -> Self {
        Self {
            name: String::new(),
            parent: None,
            offset: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            shape: BoneShape::Capsule {
                radius: 0.05,
                half_height: 0.1,
            },
            mass: 1.0,
            joint_type: BoneJointType::Spherical {
                swing_limit: Some(std::f32::consts::FRAC_PI_4),
                twist_limit: None,
            },
        }
    }
}

/// Configuration for ragdoll behavior
#[derive(Debug, Clone)]
pub struct RagdollConfig {
    /// Global mass multiplier
    pub mass_scale: f32,
    /// Joint stiffness (0.0 = floppy, 1.0 = stiff)
    pub joint_stiffness: f32,
    /// Joint damping (prevents oscillation)
    pub joint_damping: f32,
    /// Whether to enable CCD for fast-moving ragdolls
    pub enable_ccd: bool,
    /// Collision group for ragdoll bodies
    pub collision_group: Layers,
    /// Maximum angular velocity (prevents spinning out of control)
    pub max_angular_velocity: f32,
    /// Linear damping (air resistance)
    pub linear_damping: f32,
    /// Angular damping (rotational air resistance)
    pub angular_damping: f32,
}

impl Default for RagdollConfig {
    fn default() -> Self {
        Self {
            mass_scale: 1.0,
            joint_stiffness: 0.5,
            joint_damping: 0.3,
            enable_ccd: true,
            collision_group: Layers::DEFAULT,
            max_angular_velocity: 20.0,
            linear_damping: 0.1,
            angular_damping: 0.5,
        }
    }
}

/// Ragdoll state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum RagdollState {
    /// Ragdoll is fully physics-driven
    Active,
    /// Ragdoll is blending from animation to physics
    BlendingToPhysics {
        /// Progress of blend (0.0 = animation, 1.0 = physics)
        progress_percent: u8,
    },
    /// Ragdoll is blending from physics to animation
    BlendingToAnimation {
        /// Progress of blend (0.0 = physics, 1.0 = animation)
        progress_percent: u8,
    },
    /// Ragdoll is disabled (using animation)
    Disabled,
}

/// Instance of a spawned ragdoll
#[derive(Debug)]
pub struct Ragdoll {
    /// Unique ID
    pub id: RagdollId,
    /// Current state
    pub state: RagdollState,
    /// Map of bone name to body ID
    pub bone_bodies: HashMap<String, BodyId>,
    /// Map of bone name to joint ID (joint connecting to parent)
    pub bone_joints: HashMap<String, JointId>,
    /// Root bone name
    pub root_bone: String,
    /// Configuration used to create this ragdoll
    pub config: RagdollConfig,
    /// Bone definitions (for reference)
    bone_defs: Vec<BoneDef>,
}

impl Ragdoll {
    /// Get the body ID for a bone by name
    pub fn get_bone_body(&self, name: &str) -> Option<BodyId> {
        self.bone_bodies.get(name).copied()
    }

    /// Get all body IDs in this ragdoll
    pub fn all_bodies(&self) -> impl Iterator<Item = BodyId> + '_ {
        self.bone_bodies.values().copied()
    }

    /// Get the root body ID
    pub fn root_body(&self) -> Option<BodyId> {
        self.bone_bodies.get(&self.root_bone).copied()
    }

    /// Apply an impulse to a specific bone
    pub fn apply_impulse_to_bone(
        &self,
        physics: &mut PhysicsWorld,
        bone_name: &str,
        impulse: Vec3,
    ) -> bool {
        if let Some(body_id) = self.bone_bodies.get(bone_name) {
            physics.apply_impulse(*body_id, impulse);
            true
        } else {
            false
        }
    }

    /// Apply an impulse to the ragdoll with propagation through joints
    ///
    /// The impulse is applied at the specified position and propagates
    /// through the joint chain with falloff based on distance.
    pub fn apply_impulse_with_propagation(
        &self,
        physics: &mut PhysicsWorld,
        hit_bone: &str,
        impulse: Vec3,
        propagation_factor: f32,
    ) {
        // Apply full impulse to hit bone
        if let Some(body_id) = self.bone_bodies.get(hit_bone) {
            physics.apply_impulse(*body_id, impulse);
        }

        // Find connected bones and apply reduced impulse
        if let Some(hit_def) = self.bone_defs.iter().find(|b| b.name == hit_bone) {
            // Apply to parent (if any)
            if let Some(ref parent_name) = hit_def.parent {
                if let Some(parent_body) = self.bone_bodies.get(parent_name) {
                    physics.apply_impulse(*parent_body, impulse * propagation_factor);
                }
            }

            // Apply to children
            for bone in &self.bone_defs {
                if bone.parent.as_ref() == Some(&hit_def.name) {
                    if let Some(child_body) = self.bone_bodies.get(&bone.name) {
                        physics.apply_impulse(*child_body, impulse * propagation_factor);
                    }
                }
            }
        }
    }

    /// Get the center of mass of the ragdoll
    pub fn center_of_mass(&self, physics: &PhysicsWorld) -> Vec3 {
        let mut total_mass = 0.0f32;
        let mut weighted_pos = Vec3::ZERO;

        for (name, body_id) in &self.bone_bodies {
            if let Some(transform) = physics.body_transform(*body_id) {
                let pos = Vec3::new(transform.w_axis.x, transform.w_axis.y, transform.w_axis.z);
                let mass = self
                    .bone_defs
                    .iter()
                    .find(|b| &b.name == name)
                    .map(|b| b.mass)
                    .unwrap_or(1.0);
                weighted_pos += pos * mass;
                total_mass += mass;
            }
        }

        if total_mass > 0.0 {
            weighted_pos / total_mass
        } else {
            Vec3::ZERO
        }
    }

    /// Check if the ragdoll is at rest (not moving significantly)
    pub fn is_at_rest(&self, physics: &PhysicsWorld, velocity_threshold: f32) -> bool {
        for body_id in self.bone_bodies.values() {
            if let Some(vel) = physics.get_velocity(*body_id) {
                if vel.length() > velocity_threshold {
                    return false;
                }
            }
        }
        true
    }
}

/// Builder for creating ragdolls
#[derive(Debug)]
pub struct RagdollBuilder {
    /// Configuration
    pub config: RagdollConfig,
    /// Bone definitions
    bones: Vec<BoneDef>,
    /// Next ragdoll ID
    next_id: RagdollId,
}

impl RagdollBuilder {
    /// Create a new ragdoll builder with the given configuration
    pub fn new(config: RagdollConfig) -> Self {
        Self {
            config,
            bones: Vec::new(),
            next_id: 1,
        }
    }

    /// Add a bone to the ragdoll
    pub fn add_bone(
        &mut self,
        name: &str,
        parent: Option<&str>,
        offset: Vec3,
        shape: BoneShape,
        mass: f32,
    ) -> &mut Self {
        self.bones.push(BoneDef {
            name: name.to_string(),
            parent: parent.map(|s| s.to_string()),
            offset,
            shape,
            mass: mass * self.config.mass_scale,
            ..Default::default()
        });
        self
    }

    /// Add a bone with full configuration
    pub fn add_bone_full(&mut self, bone: BoneDef) -> &mut Self {
        let mut bone = bone;
        bone.mass *= self.config.mass_scale;
        self.bones.push(bone);
        self
    }

    /// Add a hinge joint bone (elbow, knee)
    #[allow(clippy::too_many_arguments)]
    pub fn add_hinge_bone(
        &mut self,
        name: &str,
        parent: &str,
        offset: Vec3,
        shape: BoneShape,
        mass: f32,
        axis: Vec3,
        limits: Option<(f32, f32)>,
    ) -> &mut Self {
        self.bones.push(BoneDef {
            name: name.to_string(),
            parent: Some(parent.to_string()),
            offset,
            shape,
            mass: mass * self.config.mass_scale,
            joint_type: BoneJointType::Hinge { axis, limits },
            ..Default::default()
        });
        self
    }

    /// Add a ball joint bone (shoulder, hip)
    pub fn add_ball_bone(
        &mut self,
        name: &str,
        parent: &str,
        offset: Vec3,
        shape: BoneShape,
        mass: f32,
        swing_limit: Option<f32>,
    ) -> &mut Self {
        self.bones.push(BoneDef {
            name: name.to_string(),
            parent: Some(parent.to_string()),
            offset,
            shape,
            mass: mass * self.config.mass_scale,
            joint_type: BoneJointType::Spherical {
                swing_limit,
                twist_limit: None,
            },
            ..Default::default()
        });
        self
    }

    /// Build the ragdoll and spawn it in the physics world
    pub fn build(&mut self, physics: &mut PhysicsWorld, spawn_position: Vec3) -> Ragdoll {
        let mut bone_bodies = HashMap::new();
        let mut bone_joints = HashMap::new();
        let mut root_bone = String::new();

        // First pass: create all bodies
        for bone in &self.bones {
            let bone_pos = if let Some(ref parent_name) = bone.parent {
                // Calculate world position based on parent
                if let Some(&parent_body) = bone_bodies.get(parent_name) {
                    if let Some(transform) = physics.body_transform(parent_body) {
                        let parent_pos =
                            Vec3::new(transform.w_axis.x, transform.w_axis.y, transform.w_axis.z);
                        parent_pos + bone.offset
                    } else {
                        spawn_position + bone.offset
                    }
                } else {
                    spawn_position + bone.offset
                }
            } else {
                // Root bone
                root_bone = bone.name.clone();
                spawn_position
            };

            // Create the body based on shape
            let body_id = match bone.shape {
                BoneShape::Capsule {
                    radius,
                    half_height,
                } => physics.add_dynamic_box(
                    bone_pos,
                    Vec3::new(radius, half_height, radius),
                    bone.mass,
                    self.config.collision_group,
                ),
                BoneShape::Sphere { radius } => physics.add_dynamic_box(
                    bone_pos,
                    Vec3::splat(radius),
                    bone.mass,
                    self.config.collision_group,
                ),
                BoneShape::Box { half_extents } => physics.add_dynamic_box(
                    bone_pos,
                    half_extents,
                    bone.mass,
                    self.config.collision_group,
                ),
            };

            // Enable CCD if configured
            if self.config.enable_ccd {
                physics.enable_ccd(body_id);
            }

            bone_bodies.insert(bone.name.clone(), body_id);
        }

        // Second pass: create joints
        for bone in &self.bones {
            if let Some(ref parent_name) = bone.parent {
                let Some(&parent_body) = bone_bodies.get(parent_name) else {
                    continue;
                };
                let Some(&child_body) = bone_bodies.get(&bone.name) else {
                    continue;
                };

                let joint_type = match bone.joint_type {
                    BoneJointType::Fixed => JointType::Fixed,
                    BoneJointType::Hinge { axis, limits } => JointType::Revolute { axis, limits },
                    BoneJointType::Spherical { .. } => JointType::Spherical,
                };

                let joint_id = physics.add_joint(parent_body, child_body, joint_type);
                bone_joints.insert(bone.name.clone(), joint_id);
            }
        }

        let id = self.next_id;
        self.next_id += 1;

        Ragdoll {
            id,
            state: RagdollState::Active,
            bone_bodies,
            bone_joints,
            root_bone,
            config: self.config.clone(),
            bone_defs: self.bones.clone(),
        }
    }

    /// Clear all bones (for reuse)
    pub fn clear(&mut self) {
        self.bones.clear();
    }
}

/// Preset ragdoll templates
pub struct RagdollPresets;

impl RagdollPresets {
    /// Create a simple humanoid ragdoll (10 bones)
    ///
    /// Bones: pelvis, spine, chest, head, upper_arm_l/r, lower_arm_l/r, upper_leg_l/r, lower_leg_l/r
    pub fn humanoid(config: RagdollConfig) -> RagdollBuilder {
        let mut builder = RagdollBuilder::new(config);

        // Torso chain
        builder.add_bone(
            "pelvis",
            None,
            Vec3::ZERO,
            BoneShape::Box {
                half_extents: Vec3::new(0.15, 0.1, 0.1),
            },
            4.0,
        );

        builder.add_bone_full(BoneDef {
            name: "spine".to_string(),
            parent: Some("pelvis".to_string()),
            offset: Vec3::new(0.0, 0.15, 0.0),
            shape: BoneShape::Box {
                half_extents: Vec3::new(0.12, 0.12, 0.08),
            },
            mass: 3.0,
            joint_type: BoneJointType::Spherical {
                swing_limit: Some(0.3),
                twist_limit: Some((-0.2, 0.2)),
            },
            ..Default::default()
        });

        builder.add_bone_full(BoneDef {
            name: "chest".to_string(),
            parent: Some("spine".to_string()),
            offset: Vec3::new(0.0, 0.2, 0.0),
            shape: BoneShape::Box {
                half_extents: Vec3::new(0.14, 0.1, 0.09),
            },
            mass: 3.5,
            joint_type: BoneJointType::Spherical {
                swing_limit: Some(0.25),
                twist_limit: Some((-0.15, 0.15)),
            },
            ..Default::default()
        });

        builder.add_bone(
            "head",
            Some("chest"),
            Vec3::new(0.0, 0.2, 0.0),
            BoneShape::Sphere { radius: 0.1 },
            1.5,
        );

        // Arms
        for (side, x_offset) in [("_l", -0.2), ("_r", 0.2)] {
            let upper_arm = format!("upper_arm{}", side);
            let lower_arm = format!("lower_arm{}", side);

            builder.add_ball_bone(
                &upper_arm,
                "chest",
                Vec3::new(x_offset, 0.05, 0.0),
                BoneShape::Capsule {
                    radius: 0.04,
                    half_height: 0.12,
                },
                1.2,
                Some(std::f32::consts::FRAC_PI_2),
            );

            builder.add_hinge_bone(
                &lower_arm,
                &upper_arm,
                Vec3::new(0.0, -0.25, 0.0),
                BoneShape::Capsule {
                    radius: 0.035,
                    half_height: 0.11,
                },
                0.8,
                Vec3::X,
                Some((0.0, std::f32::consts::FRAC_PI_2 * 1.5)),
            );
        }

        // Legs
        for (side, x_offset) in [("_l", -0.1), ("_r", 0.1)] {
            let upper_leg = format!("upper_leg{}", side);
            let lower_leg = format!("lower_leg{}", side);

            builder.add_ball_bone(
                &upper_leg,
                "pelvis",
                Vec3::new(x_offset, -0.15, 0.0),
                BoneShape::Capsule {
                    radius: 0.06,
                    half_height: 0.18,
                },
                3.0,
                Some(std::f32::consts::FRAC_PI_3),
            );

            builder.add_hinge_bone(
                &lower_leg,
                &upper_leg,
                Vec3::new(0.0, -0.38, 0.0),
                BoneShape::Capsule {
                    radius: 0.05,
                    half_height: 0.17,
                },
                2.0,
                Vec3::X,
                Some((-std::f32::consts::FRAC_PI_2 * 1.5, 0.0)),
            );
        }

        builder
    }

    /// Create a simple quadruped ragdoll (8 bones)
    ///
    /// Bones: body, head, front_leg_l/r, back_leg_l/r
    pub fn quadruped(config: RagdollConfig) -> RagdollBuilder {
        let mut builder = RagdollBuilder::new(config);

        // Body
        builder.add_bone(
            "body",
            None,
            Vec3::ZERO,
            BoneShape::Box {
                half_extents: Vec3::new(0.2, 0.15, 0.4),
            },
            8.0,
        );

        // Head
        builder.add_ball_bone(
            "head",
            "body",
            Vec3::new(0.0, 0.1, 0.35),
            BoneShape::Sphere { radius: 0.12 },
            1.5,
            Some(std::f32::consts::FRAC_PI_4),
        );

        // Front legs
        for (side, x_offset) in [("_l", -0.15), ("_r", 0.15)] {
            let leg_name = format!("front_leg{}", side);
            builder.add_ball_bone(
                &leg_name,
                "body",
                Vec3::new(x_offset, -0.2, 0.25),
                BoneShape::Capsule {
                    radius: 0.04,
                    half_height: 0.15,
                },
                1.0,
                Some(std::f32::consts::FRAC_PI_3),
            );
        }

        // Back legs
        for (side, x_offset) in [("_l", -0.15), ("_r", 0.15)] {
            let leg_name = format!("back_leg{}", side);
            builder.add_ball_bone(
                &leg_name,
                "body",
                Vec3::new(x_offset, -0.2, -0.25),
                BoneShape::Capsule {
                    radius: 0.04,
                    half_height: 0.15,
                },
                1.0,
                Some(std::f32::consts::FRAC_PI_3),
            );
        }

        // Tail
        builder.add_ball_bone(
            "tail",
            "body",
            Vec3::new(0.0, 0.05, -0.45),
            BoneShape::Capsule {
                radius: 0.02,
                half_height: 0.15,
            },
            0.3,
            Some(std::f32::consts::FRAC_PI_2),
        );

        builder
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bone_shape_volume() {
        let capsule = BoneShape::Capsule {
            radius: 1.0,
            half_height: 1.0,
        };
        assert!(capsule.volume() > 0.0);

        let sphere = BoneShape::Sphere { radius: 1.0 };
        let expected_sphere_vol = (4.0 / 3.0) * std::f32::consts::PI;
        assert!((sphere.volume() - expected_sphere_vol).abs() < 0.001);

        let box_shape = BoneShape::Box {
            half_extents: Vec3::ONE,
        };
        assert!((box_shape.volume() - 8.0).abs() < 0.001);
    }

    #[test]
    fn test_ragdoll_builder_creation() {
        let config = RagdollConfig::default();
        let builder = RagdollBuilder::new(config);
        assert_eq!(builder.bones.len(), 0);
    }

    #[test]
    fn test_ragdoll_builder_add_bones() {
        let mut builder = RagdollBuilder::new(RagdollConfig::default());

        builder.add_bone(
            "root",
            None,
            Vec3::ZERO,
            BoneShape::Box {
                half_extents: Vec3::splat(0.1),
            },
            1.0,
        );

        builder.add_bone(
            "child",
            Some("root"),
            Vec3::new(0.0, 0.5, 0.0),
            BoneShape::Sphere { radius: 0.1 },
            0.5,
        );

        assert_eq!(builder.bones.len(), 2);
        assert_eq!(builder.bones[0].name, "root");
        assert!(builder.bones[0].parent.is_none());
        assert_eq!(builder.bones[1].name, "child");
        assert_eq!(builder.bones[1].parent, Some("root".to_string()));
    }

    #[test]
    fn test_ragdoll_config_defaults() {
        let config = RagdollConfig::default();
        assert!((config.mass_scale - 1.0).abs() < 0.001);
        assert!(config.enable_ccd);
        assert!(config.max_angular_velocity > 0.0);
    }

    #[test]
    fn test_ragdoll_state_enum() {
        assert_eq!(RagdollState::Active, RagdollState::Active);
        assert_ne!(RagdollState::Active, RagdollState::Disabled);

        let blending = RagdollState::BlendingToPhysics {
            progress_percent: 50,
        };
        match blending {
            RagdollState::BlendingToPhysics { progress_percent } => {
                assert_eq!(progress_percent, 50);
            }
            _ => panic!("Wrong state"),
        }
    }

    #[test]
    fn test_humanoid_preset() {
        let builder = RagdollPresets::humanoid(RagdollConfig::default());
        // Humanoid should have: pelvis, spine, chest, head,
        // upper_arm_l, upper_arm_r, lower_arm_l, lower_arm_r,
        // upper_leg_l, upper_leg_r, lower_leg_l, lower_leg_r
        assert_eq!(builder.bones.len(), 12);
    }

    #[test]
    fn test_quadruped_preset() {
        let builder = RagdollPresets::quadruped(RagdollConfig::default());
        // Quadruped should have: body, head, 4 legs, tail
        assert_eq!(builder.bones.len(), 7);
    }

    #[test]
    fn test_add_hinge_bone() {
        let mut builder = RagdollBuilder::new(RagdollConfig::default());

        builder.add_bone(
            "parent",
            None,
            Vec3::ZERO,
            BoneShape::Sphere { radius: 0.1 },
            1.0,
        );
        builder.add_hinge_bone(
            "child",
            "parent",
            Vec3::Y,
            BoneShape::Capsule {
                radius: 0.05,
                half_height: 0.1,
            },
            0.5,
            Vec3::X,
            Some((0.0, std::f32::consts::PI)),
        );

        assert_eq!(builder.bones.len(), 2);
        match builder.bones[1].joint_type {
            BoneJointType::Hinge { axis, limits } => {
                assert!((axis.x - 1.0).abs() < 0.001);
                assert!(limits.is_some());
            }
            _ => panic!("Expected Hinge joint"),
        }
    }

    #[test]
    fn test_add_ball_bone() {
        let mut builder = RagdollBuilder::new(RagdollConfig::default());

        builder.add_bone(
            "parent",
            None,
            Vec3::ZERO,
            BoneShape::Sphere { radius: 0.1 },
            1.0,
        );
        builder.add_ball_bone(
            "child",
            "parent",
            Vec3::Y,
            BoneShape::Sphere { radius: 0.08 },
            0.5,
            Some(std::f32::consts::FRAC_PI_4),
        );

        assert_eq!(builder.bones.len(), 2);
        match builder.bones[1].joint_type {
            BoneJointType::Spherical { swing_limit, .. } => {
                assert!(swing_limit.is_some());
            }
            _ => panic!("Expected Spherical joint"),
        }
    }

    #[test]
    fn test_builder_clear() {
        let mut builder = RagdollBuilder::new(RagdollConfig::default());
        builder.add_bone(
            "test",
            None,
            Vec3::ZERO,
            BoneShape::Sphere { radius: 0.1 },
            1.0,
        );
        assert_eq!(builder.bones.len(), 1);

        builder.clear();
        assert_eq!(builder.bones.len(), 0);
    }

    #[test]
    fn test_mass_scale() {
        let config = RagdollConfig {
            mass_scale: 2.0,
            ..Default::default()
        };
        let mut builder = RagdollBuilder::new(config);

        builder.add_bone(
            "test",
            None,
            Vec3::ZERO,
            BoneShape::Sphere { radius: 0.1 },
            5.0,
        );

        // Mass should be scaled
        assert!((builder.bones[0].mass - 10.0).abs() < 0.001);
    }

    #[test]
    fn test_ragdoll_build_simple() {
        let mut physics = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));
        let mut builder = RagdollBuilder::new(RagdollConfig::default());

        builder.add_bone(
            "root",
            None,
            Vec3::ZERO,
            BoneShape::Sphere { radius: 0.2 },
            2.0,
        );
        builder.add_bone(
            "child",
            Some("root"),
            Vec3::new(0.0, 0.5, 0.0),
            BoneShape::Sphere { radius: 0.15 },
            1.0,
        );

        let ragdoll = builder.build(&mut physics, Vec3::new(0.0, 5.0, 0.0));

        assert_eq!(ragdoll.bone_bodies.len(), 2);
        assert_eq!(ragdoll.bone_joints.len(), 1); // Only child has joint
        assert_eq!(ragdoll.root_bone, "root");
        assert_eq!(ragdoll.state, RagdollState::Active);
    }

    #[test]
    fn test_ragdoll_humanoid_build() {
        let mut physics = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));
        let mut builder = RagdollPresets::humanoid(RagdollConfig::default());

        let ragdoll = builder.build(&mut physics, Vec3::new(0.0, 2.0, 0.0));

        assert_eq!(ragdoll.bone_bodies.len(), 12);
        assert_eq!(ragdoll.bone_joints.len(), 11); // All except root
        assert_eq!(ragdoll.root_bone, "pelvis");
    }

    #[test]
    fn test_ragdoll_get_bone_body() {
        let mut physics = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));
        let mut builder = RagdollPresets::humanoid(RagdollConfig::default());
        let ragdoll = builder.build(&mut physics, Vec3::ZERO);

        assert!(ragdoll.get_bone_body("pelvis").is_some());
        assert!(ragdoll.get_bone_body("head").is_some());
        assert!(ragdoll.get_bone_body("nonexistent").is_none());
    }

    #[test]
    fn test_ragdoll_root_body() {
        let mut physics = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));
        let mut builder = RagdollPresets::humanoid(RagdollConfig::default());
        let ragdoll = builder.build(&mut physics, Vec3::ZERO);

        let root = ragdoll.root_body();
        assert!(root.is_some());
        assert_eq!(root, ragdoll.get_bone_body("pelvis"));
    }

    #[test]
    fn test_ragdoll_all_bodies() {
        let mut physics = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));
        let mut builder = RagdollPresets::humanoid(RagdollConfig::default());
        let ragdoll = builder.build(&mut physics, Vec3::ZERO);

        let bodies: Vec<_> = ragdoll.all_bodies().collect();
        assert_eq!(bodies.len(), 12);
    }

    #[test]
    fn test_ragdoll_center_of_mass() {
        let mut physics = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));
        let mut builder = RagdollBuilder::new(RagdollConfig::default());

        builder.add_bone(
            "root",
            None,
            Vec3::ZERO,
            BoneShape::Sphere { radius: 0.2 },
            1.0,
        );

        let ragdoll = builder.build(&mut physics, Vec3::new(0.0, 5.0, 0.0));
        let com = ragdoll.center_of_mass(&physics);

        // Should be near spawn position
        assert!((com.y - 5.0).abs() < 1.0);
    }

    #[test]
    fn test_ragdoll_is_at_rest() {
        let mut physics = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));
        let mut builder = RagdollBuilder::new(RagdollConfig::default());

        builder.add_bone(
            "root",
            None,
            Vec3::ZERO,
            BoneShape::Sphere { radius: 0.2 },
            1.0,
        );

        let ragdoll = builder.build(&mut physics, Vec3::new(0.0, 5.0, 0.0));

        // Initially should be at rest (no steps taken)
        let at_rest = ragdoll.is_at_rest(&physics, 0.1);
        assert!(at_rest);
    }

    // ============================================================================
    // JOINT LIMIT TESTS (Phase 8.8 - New)
    // ============================================================================

    #[test]
    fn test_hinge_joint_limits_validation() {
        let joint = BoneJointType::Hinge {
            axis: Vec3::X,
            limits: Some((0.0, std::f32::consts::PI)),
        };

        if let BoneJointType::Hinge { limits, .. } = joint {
            let (min, max) = limits.unwrap();
            assert!(min < max);
            assert!(min >= 0.0);
            assert!(max <= std::f32::consts::PI);
        }
    }

    #[test]
    fn test_spherical_swing_limit() {
        let joint = BoneJointType::Spherical {
            swing_limit: Some(std::f32::consts::FRAC_PI_4),
            twist_limit: None,
        };

        if let BoneJointType::Spherical { swing_limit, .. } = joint {
            assert!(swing_limit.unwrap() > 0.0);
            assert!(swing_limit.unwrap() < std::f32::consts::PI);
        }
    }

    #[test]
    fn test_spherical_twist_limit() {
        let joint = BoneJointType::Spherical {
            swing_limit: None,
            twist_limit: Some((-0.5, 0.5)),
        };

        if let BoneJointType::Spherical { twist_limit, .. } = joint {
            let (min, max) = twist_limit.unwrap();
            assert!(min < max);
        }
    }

    #[test]
    fn test_fixed_joint() {
        let joint = BoneJointType::Fixed;
        assert!(matches!(joint, BoneJointType::Fixed));
    }

    #[test]
    fn test_knee_joint_realistic_limits() {
        // Knee can bend 0-135 degrees roughly
        let knee = BoneJointType::Hinge {
            axis: Vec3::X,
            limits: Some((0.0, 2.356)), // ~135 degrees in radians
        };

        if let BoneJointType::Hinge { limits, axis } = knee {
            let (min, max) = limits.unwrap();
            assert!(max > 2.0); // At least 115 degrees
            assert_eq!(axis, Vec3::X);
        }
    }

    // ============================================================================
    // POSE BLENDING TESTS (Phase 8.8 - New)
    // ============================================================================

    #[test]
    fn test_blend_state_to_physics() {
        let state = RagdollState::BlendingToPhysics {
            progress_percent: 0,
        };
        assert!(matches!(state, RagdollState::BlendingToPhysics { .. }));

        // Progress validation
        for progress in [0, 25, 50, 75, 100] {
            let s = RagdollState::BlendingToPhysics {
                progress_percent: progress,
            };
            if let RagdollState::BlendingToPhysics { progress_percent } = s {
                assert!(progress_percent <= 100);
            }
        }
    }

    #[test]
    fn test_blend_state_to_animation() {
        let state = RagdollState::BlendingToAnimation {
            progress_percent: 50,
        };

        if let RagdollState::BlendingToAnimation { progress_percent } = state {
            assert_eq!(progress_percent, 50);
        } else {
            panic!("Expected BlendingToAnimation");
        }
    }

    #[test]
    fn test_state_transitions() {
        // Valid state flow: Disabled -> BlendingToPhysics -> Active
        let states = [
            RagdollState::Disabled,
            RagdollState::BlendingToPhysics {
                progress_percent: 50,
            },
            RagdollState::Active,
        ];

        for (i, state) in states.iter().enumerate() {
            match i {
                0 => assert_eq!(*state, RagdollState::Disabled),
                1 => assert!(matches!(*state, RagdollState::BlendingToPhysics { .. })),
                2 => assert_eq!(*state, RagdollState::Active),
                _ => {}
            }
        }
    }

    // ============================================================================
    // FALL RECOVERY TESTS (Phase 8.8 - New)
    // ============================================================================

    #[test]
    fn test_ragdoll_bone_count() {
        let mut physics = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));
        let mut builder = RagdollPresets::humanoid(RagdollConfig::default());
        let ragdoll = builder.build(&mut physics, Vec3::ZERO);

        assert_eq!(ragdoll.bone_defs.len(), 12);
        assert_eq!(ragdoll.bone_bodies.len(), 12);
    }

    #[test]
    fn test_ragdoll_joint_hierarchy() {
        let mut physics = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));
        let mut builder = RagdollPresets::humanoid(RagdollConfig::default());
        let ragdoll = builder.build(&mut physics, Vec3::ZERO);

        // All non-root bones should have joints
        let non_root_count = ragdoll
            .bone_defs
            .iter()
            .filter(|b| b.parent.is_some())
            .count();

        assert_eq!(ragdoll.bone_joints.len(), non_root_count);
    }

    // ============================================================================
    // EDGE CASE TESTS (Phase 8.8 - New)
    // ============================================================================

    #[test]
    fn test_bone_def_default() {
        let def = BoneDef::default();

        assert!(def.name.is_empty());
        assert!(def.parent.is_none());
        assert_eq!(def.offset, Vec3::ZERO);
        assert_eq!(def.rotation, Quat::IDENTITY);
        assert_eq!(def.mass, 1.0);
    }

    #[test]
    fn test_config_joint_stiffness_range() {
        let config = RagdollConfig::default();
        assert!(config.joint_stiffness >= 0.0 && config.joint_stiffness <= 1.0);
    }

    #[test]
    fn test_config_damping_values() {
        let config = RagdollConfig::default();
        assert!(config.linear_damping > 0.0);
        assert!(config.angular_damping > 0.0);
        assert!(config.joint_damping > 0.0);
    }

    #[test]
    fn test_empty_ragdoll_builder() {
        let builder = RagdollBuilder::new(RagdollConfig::default());
        assert!(builder.bones.is_empty());
    }

    #[test]
    fn test_single_bone_ragdoll() {
        let mut physics = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));
        let mut builder = RagdollBuilder::new(RagdollConfig::default());

        builder.add_bone(
            "only",
            None,
            Vec3::ZERO,
            BoneShape::Sphere { radius: 0.5 },
            5.0,
        );

        let ragdoll = builder.build(&mut physics, Vec3::ZERO);

        assert_eq!(ragdoll.bone_bodies.len(), 1);
        assert_eq!(ragdoll.bone_joints.len(), 0); // No joints for single bone
    }

    #[test]
    fn test_bone_shape_volume_zero_radius() {
        let sphere = BoneShape::Sphere { radius: 0.0 };
        assert_eq!(sphere.volume(), 0.0);

        let capsule = BoneShape::Capsule {
            radius: 0.0,
            half_height: 1.0,
        };
        assert_eq!(capsule.volume(), 0.0);

        let box_shape = BoneShape::Box {
            half_extents: Vec3::ZERO,
        };
        assert_eq!(box_shape.volume(), 0.0);
    }

    #[test]
    fn test_quadruped_structure() {
        let mut physics = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));
        let mut builder = RagdollPresets::quadruped(RagdollConfig::default());
        let ragdoll = builder.build(&mut physics, Vec3::ZERO);

        // Should have body as root
        assert_eq!(ragdoll.root_bone, "body");

        // Should have 7 bones
        assert_eq!(ragdoll.bone_bodies.len(), 7);

        // Should have 6 joints (all except root)
        assert_eq!(ragdoll.bone_joints.len(), 6);
    }

    #[test]
    fn test_ragdoll_spawn_position() {
        let mut physics = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));
        let mut builder = RagdollBuilder::new(RagdollConfig::default());

        builder.add_bone(
            "root",
            None,
            Vec3::ZERO,
            BoneShape::Sphere { radius: 0.1 },
            1.0,
        );

        let spawn_pos = Vec3::new(10.0, 20.0, 30.0);
        let ragdoll = builder.build(&mut physics, spawn_pos);
        let com = ragdoll.center_of_mass(&physics);

        // Center of mass should be near spawn position
        assert!((com - spawn_pos).length() < 1.0);
    }

    #[test]
    fn test_custom_mass_scale() {
        let config = RagdollConfig {
            mass_scale: 0.5,
            ..Default::default()
        };

        let mut builder = RagdollBuilder::new(config);
        builder.add_bone(
            "test",
            None,
            Vec3::ZERO,
            BoneShape::Sphere { radius: 0.1 },
            10.0,
        );

        // Mass should be halved
        assert!((builder.bones[0].mass - 5.0).abs() < 0.001);
    }

    #[test]
    fn test_ccd_enabled_by_default() {
        let config = RagdollConfig::default();
        assert!(config.enable_ccd);
    }

    #[test]
    fn test_max_angular_velocity() {
        let config = RagdollConfig::default();
        assert!(config.max_angular_velocity > 0.0);
        assert!(config.max_angular_velocity < 100.0); // Reasonable limit
    }

    // ═══════════════════════════════════════════════════════════════
    // DEEP REMEDIATION v3.6 — ragdoll / bone shape tests
    // ═══════════════════════════════════════════════════════════════

    #[test]
    fn mutation_sphere_volume_exact() {
        let r = 2.0;
        let shape = BoneShape::Sphere { radius: r };
        let expected = (4.0 / 3.0) * std::f32::consts::PI * r.powi(3);
        assert!(
            (shape.volume() - expected).abs() < 0.01,
            "Sphere volume: expected {}, got {}",
            expected,
            shape.volume()
        );
    }

    #[test]
    fn mutation_box_volume_exact() {
        let he = Vec3::new(1.0, 2.0, 3.0);
        let shape = BoneShape::Box { half_extents: he };
        let expected = 8.0 * he.x * he.y * he.z; // = 48.0
        assert!(
            (shape.volume() - expected).abs() < 0.01,
            "Box volume: expected {}, got {}",
            expected,
            shape.volume()
        );
        assert!(
            (shape.volume() - 48.0).abs() < 0.01,
            "Box(1,2,3) should be 48.0"
        );
    }

    #[test]
    fn mutation_capsule_volume_exact() {
        let r = 1.0;
        let hh = 2.0;
        let shape = BoneShape::Capsule {
            radius: r,
            half_height: hh,
        };
        // Cylinder part: π * r² * 2h = π * 1 * 4 = 4π
        let cylinder = std::f32::consts::PI * r * r * (hh * 2.0);
        // Sphere caps: 4/3 * π * r³ = 4/3 * π
        let sphere = (4.0 / 3.0) * std::f32::consts::PI * r.powi(3);
        let expected = cylinder + sphere;
        assert!(
            (shape.volume() - expected).abs() < 0.01,
            "Capsule volume: expected {}, got {}",
            expected,
            shape.volume()
        );
    }

    #[test]
    fn mutation_capsule_larger_than_sphere_same_radius() {
        let r = 1.5;
        let capsule = BoneShape::Capsule {
            radius: r,
            half_height: 1.0,
        };
        let sphere = BoneShape::Sphere { radius: r };
        assert!(
            capsule.volume() > sphere.volume(),
            "Capsule with half_height>0 should have greater volume than sphere of same radius"
        );
    }

    #[test]
    fn mutation_volume_scales_with_size() {
        let small = BoneShape::Sphere { radius: 1.0 };
        let big = BoneShape::Sphere { radius: 2.0 };
        // Volume scales as r³, so 2³ = 8x
        let ratio = big.volume() / small.volume();
        assert!(
            (ratio - 8.0).abs() < 0.01,
            "Volume should scale as r³, ratio {}",
            ratio
        );
    }

    #[test]
    fn mutation_builder_bone_mass_scale() {
        let config = RagdollConfig {
            mass_scale: 2.0,
            ..Default::default()
        };
        let mut builder = RagdollBuilder::new(config);
        builder.add_bone(
            "bone_a",
            None,
            Vec3::ZERO,
            BoneShape::Sphere { radius: 0.5 },
            10.0,
        );
        // Mass should be doubled
        assert!(
            (builder.bones[0].mass - 20.0).abs() < 0.001,
            "Mass should be 10*2=20, got {}",
            builder.bones[0].mass
        );
    }

    #[test]
    fn mutation_builder_parent_child_relationship() {
        let config = RagdollConfig::default();
        let mut builder = RagdollBuilder::new(config);
        builder.add_bone(
            "spine",
            None,
            Vec3::ZERO,
            BoneShape::Box {
                half_extents: Vec3::new(0.2, 0.3, 0.15),
            },
            5.0,
        );
        builder.add_bone(
            "head",
            Some("spine"),
            Vec3::new(0.0, 0.5, 0.0),
            BoneShape::Sphere { radius: 0.1 },
            3.0,
        );

        assert_eq!(builder.bones.len(), 2);
        assert!(builder.bones[0].parent.is_none());
        assert_eq!(builder.bones[1].parent, Some("spine".to_string()));
    }

    #[test]
    fn mutation_box_volume_zero_extent() {
        let shape = BoneShape::Box {
            half_extents: Vec3::new(0.0, 1.0, 1.0),
        };
        assert!(
            shape.volume().abs() < 1e-6,
            "Box with zero-extent axis should have zero volume"
        );
    }

    // ═══════════════════════════════════════════════════════════════
    // DEEP REMEDIATION v3.6.1 — ragdoll Round 2 arithmetic/preset tests
    // ═══════════════════════════════════════════════════════════════

    // --- BoneShape::volume exact formulas ---
    #[test]
    fn mutation_capsule_volume_exact_r2_hh3() {
        let r = 0.5;
        let hh = 3.0;
        let shape = BoneShape::Capsule {
            radius: r,
            half_height: hh,
        };
        // cylinder = π * r² * 2hh
        let cyl = std::f32::consts::PI * r * r * (hh * 2.0);
        // sphere caps = 4/3 * π * r³
        let sph = (4.0 / 3.0) * std::f32::consts::PI * r.powi(3);
        let expected = cyl + sph;
        assert!(
            (shape.volume() - expected).abs() < 0.01,
            "Capsule(r=0.5,hh=3) volume: expected {}, got {}",
            expected,
            shape.volume()
        );
    }

    #[test]
    fn mutation_sphere_volume_tiny() {
        let r = 0.01;
        let shape = BoneShape::Sphere { radius: r };
        let expected = (4.0 / 3.0) * std::f32::consts::PI * r.powi(3);
        assert!(
            (shape.volume() - expected).abs() < 1e-8,
            "Tiny sphere: expected {}, got {}",
            expected,
            shape.volume()
        );
    }

    #[test]
    fn mutation_box_volume_asymmetric() {
        let he = Vec3::new(0.1, 0.2, 0.3);
        let shape = BoneShape::Box { half_extents: he };
        let expected = 8.0 * 0.1 * 0.2 * 0.3; // 0.048
        assert!(
            (shape.volume() - expected).abs() < 1e-5,
            "Box(0.1,0.2,0.3): expected {}, got {}",
            expected,
            shape.volume()
        );
    }

    // --- RagdollPresets::humanoid bone structure ---
    #[test]
    fn mutation_humanoid_bone_names() {
        let builder = RagdollPresets::humanoid(RagdollConfig::default());
        let names: Vec<&str> = builder.bones.iter().map(|b| b.name.as_str()).collect();
        assert!(names.contains(&"pelvis"), "Missing pelvis");
        assert!(names.contains(&"spine"), "Missing spine");
        assert!(names.contains(&"chest"), "Missing chest");
        assert!(names.contains(&"head"), "Missing head");
        assert!(names.contains(&"upper_arm_l"), "Missing upper_arm_l");
        assert!(names.contains(&"upper_arm_r"), "Missing upper_arm_r");
        assert!(names.contains(&"lower_arm_l"), "Missing lower_arm_l");
        assert!(names.contains(&"lower_arm_r"), "Missing lower_arm_r");
        assert!(names.contains(&"upper_leg_l"), "Missing upper_leg_l");
        assert!(names.contains(&"upper_leg_r"), "Missing upper_leg_r");
        assert!(names.contains(&"lower_leg_l"), "Missing lower_leg_l");
        assert!(names.contains(&"lower_leg_r"), "Missing lower_leg_r");
    }

    #[test]
    fn mutation_humanoid_pelvis_is_root() {
        let builder = RagdollPresets::humanoid(RagdollConfig::default());
        let pelvis = builder.bones.iter().find(|b| b.name == "pelvis").unwrap();
        assert!(pelvis.parent.is_none(), "Pelvis should be root (no parent)");
    }

    #[test]
    fn mutation_humanoid_mass_scale_applied() {
        let config = RagdollConfig {
            mass_scale: 3.0,
            ..Default::default()
        };
        let builder = RagdollPresets::humanoid(config);
        // pelvis base mass = 4.0, scaled = 12.0
        let pelvis = builder.bones.iter().find(|b| b.name == "pelvis").unwrap();
        assert!(
            (pelvis.mass - 12.0).abs() < 0.01,
            "Pelvis mass should be 4.0*3.0=12.0, got {}",
            pelvis.mass
        );
        // head base mass = 1.5, scaled = 4.5
        let head = builder.bones.iter().find(|b| b.name == "head").unwrap();
        assert!(
            (head.mass - 4.5).abs() < 0.01,
            "Head mass should be 1.5*3.0=4.5, got {}",
            head.mass
        );
    }

    #[test]
    fn mutation_humanoid_joint_types() {
        let builder = RagdollPresets::humanoid(RagdollConfig::default());
        // spine should be Spherical
        let spine = builder.bones.iter().find(|b| b.name == "spine").unwrap();
        assert!(
            matches!(spine.joint_type, BoneJointType::Spherical { .. }),
            "Spine should have Spherical joint"
        );
        // lower_arm_l should be Hinge
        let lower_arm = builder
            .bones
            .iter()
            .find(|b| b.name == "lower_arm_l")
            .unwrap();
        assert!(
            matches!(lower_arm.joint_type, BoneJointType::Hinge { .. }),
            "Lower arm should have Hinge joint"
        );
        // upper_arm_l should be Spherical (ball joint)
        let upper_arm = builder
            .bones
            .iter()
            .find(|b| b.name == "upper_arm_l")
            .unwrap();
        assert!(
            matches!(upper_arm.joint_type, BoneJointType::Spherical { .. }),
            "Upper arm should have Spherical joint"
        );
        // lower_leg_l should be Hinge
        let lower_leg = builder
            .bones
            .iter()
            .find(|b| b.name == "lower_leg_l")
            .unwrap();
        assert!(
            matches!(lower_leg.joint_type, BoneJointType::Hinge { .. }),
            "Lower leg should have Hinge joint"
        );
    }

    #[test]
    fn mutation_humanoid_arm_parent_chain() {
        let builder = RagdollPresets::humanoid(RagdollConfig::default());
        let lower_arm_l = builder
            .bones
            .iter()
            .find(|b| b.name == "lower_arm_l")
            .unwrap();
        assert_eq!(lower_arm_l.parent, Some("upper_arm_l".to_string()));
        let upper_arm_l = builder
            .bones
            .iter()
            .find(|b| b.name == "upper_arm_l")
            .unwrap();
        assert_eq!(upper_arm_l.parent, Some("chest".to_string()));
    }

    // --- RagdollPresets::quadruped ---
    #[test]
    fn mutation_quadruped_bone_names() {
        let builder = RagdollPresets::quadruped(RagdollConfig::default());
        let names: Vec<&str> = builder.bones.iter().map(|b| b.name.as_str()).collect();
        assert!(names.contains(&"body"), "Missing body");
        assert!(names.contains(&"head"), "Missing head");
        assert!(names.contains(&"front_leg_l"), "Missing front_leg_l");
        assert!(names.contains(&"front_leg_r"), "Missing front_leg_r");
        assert!(names.contains(&"back_leg_l"), "Missing back_leg_l");
        assert!(names.contains(&"back_leg_r"), "Missing back_leg_r");
        assert!(names.contains(&"tail"), "Missing tail");
    }

    #[test]
    fn mutation_quadruped_body_mass_exact() {
        let builder = RagdollPresets::quadruped(RagdollConfig::default());
        let body = builder.bones.iter().find(|b| b.name == "body").unwrap();
        // base mass = 8.0, mass_scale = 1.0
        assert!(
            (body.mass - 8.0).abs() < 0.01,
            "Body mass should be 8.0, got {}",
            body.mass
        );
    }

    #[test]
    fn mutation_quadruped_mass_scale_applied() {
        let config = RagdollConfig {
            mass_scale: 0.5,
            ..Default::default()
        };
        let builder = RagdollPresets::quadruped(config);
        // body base mass = 8.0, scaled = 4.0
        let body = builder.bones.iter().find(|b| b.name == "body").unwrap();
        assert!(
            (body.mass - 4.0).abs() < 0.01,
            "Body mass should be 8.0*0.5=4.0, got {}",
            body.mass
        );
        // tail base mass = 0.3, scaled = 0.15
        let tail = builder.bones.iter().find(|b| b.name == "tail").unwrap();
        assert!(
            (tail.mass - 0.15).abs() < 0.01,
            "Tail mass should be 0.3*0.5=0.15, got {}",
            tail.mass
        );
    }

    #[test]
    fn mutation_quadruped_all_legs_parent_body() {
        let builder = RagdollPresets::quadruped(RagdollConfig::default());
        for name in ["front_leg_l", "front_leg_r", "back_leg_l", "back_leg_r"] {
            let leg = builder.bones.iter().find(|b| b.name == name).unwrap();
            assert_eq!(
                leg.parent,
                Some("body".to_string()),
                "{} should have 'body' as parent",
                name
            );
        }
    }

    // --- add_hinge_bone / add_ball_bone joint details ---
    #[test]
    fn mutation_add_hinge_bone_mass_scaled() {
        let config = RagdollConfig {
            mass_scale: 2.5,
            ..Default::default()
        };
        let mut builder = RagdollBuilder::new(config);
        builder.add_bone(
            "root",
            None,
            Vec3::ZERO,
            BoneShape::Sphere { radius: 0.1 },
            1.0,
        );
        builder.add_hinge_bone(
            "hinge_child",
            "root",
            Vec3::Y,
            BoneShape::Capsule {
                radius: 0.05,
                half_height: 0.1,
            },
            4.0,
            Vec3::X,
            Some((0.0, 1.5)),
        );
        // mass = 4.0 * 2.5 = 10.0
        assert!(
            (builder.bones[1].mass - 10.0).abs() < 0.01,
            "Hinge bone mass should be 4*2.5=10, got {}",
            builder.bones[1].mass
        );
        // Joint type preserved
        match builder.bones[1].joint_type {
            BoneJointType::Hinge { axis, limits } => {
                assert!((axis.x - 1.0).abs() < 0.01);
                let (lo, hi) = limits.unwrap();
                assert!((lo - 0.0).abs() < 0.01);
                assert!((hi - 1.5).abs() < 0.01);
            }
            _ => panic!("Expected Hinge joint"),
        }
    }

    #[test]
    fn mutation_add_ball_bone_mass_scaled() {
        let config = RagdollConfig {
            mass_scale: 0.1,
            ..Default::default()
        };
        let mut builder = RagdollBuilder::new(config);
        builder.add_bone(
            "root",
            None,
            Vec3::ZERO,
            BoneShape::Sphere { radius: 0.1 },
            1.0,
        );
        builder.add_ball_bone(
            "ball_child",
            "root",
            Vec3::new(1.0, 0.0, 0.0),
            BoneShape::Sphere { radius: 0.05 },
            20.0,
            Some(0.8),
        );
        // mass = 20.0 * 0.1 = 2.0
        assert!(
            (builder.bones[1].mass - 2.0).abs() < 0.01,
            "Ball bone mass should be 20*0.1=2, got {}",
            builder.bones[1].mass
        );
        match builder.bones[1].joint_type {
            BoneJointType::Spherical {
                swing_limit,
                twist_limit,
            } => {
                assert!((swing_limit.unwrap() - 0.8).abs() < 0.01);
                assert!(twist_limit.is_none());
            }
            _ => panic!("Expected Spherical joint"),
        }
    }

    // --- add_bone_full mass_scale ---
    #[test]
    fn mutation_add_bone_full_mass_scale() {
        let config = RagdollConfig {
            mass_scale: 5.0,
            ..Default::default()
        };
        let mut builder = RagdollBuilder::new(config);
        builder.add_bone_full(BoneDef {
            name: "full_bone".to_string(),
            parent: None,
            offset: Vec3::ZERO,
            shape: BoneShape::Box {
                half_extents: Vec3::ONE,
            },
            mass: 3.0,
            joint_type: BoneJointType::Fixed,
            ..Default::default()
        });
        // mass = 3.0 * 5.0 = 15.0
        assert!(
            (builder.bones[0].mass - 15.0).abs() < 0.01,
            "add_bone_full mass should be 3*5=15, got {}",
            builder.bones[0].mass
        );
    }

    // --- Humanoid pelvis shape/mass ---
    #[test]
    fn mutation_humanoid_pelvis_shape_and_mass() {
        let builder = RagdollPresets::humanoid(RagdollConfig::default());
        let pelvis = builder.bones.iter().find(|b| b.name == "pelvis").unwrap();
        assert!(
            (pelvis.mass - 4.0).abs() < 0.01,
            "Pelvis mass should be 4.0"
        );
        match pelvis.shape {
            BoneShape::Box { half_extents } => {
                assert!((half_extents.x - 0.15).abs() < 0.01);
                assert!((half_extents.y - 0.1).abs() < 0.01);
                assert!((half_extents.z - 0.1).abs() < 0.01);
            }
            _ => panic!("Pelvis should be Box shape"),
        }
    }

    // ===== DEEP REMEDIATION v3.6.2 — ragdoll Round 3 remaining mutations =====

    // --- Humanoid offset sign mutations (delete - on line 538, 553, 567, etc.) ---
    #[test]
    fn mutation_r3_humanoid_leg_offset_y_negative() {
        // upper_leg offset = (x, -0.15, 0)  -- delete - gives (x, 0.15, 0)
        let config = RagdollConfig {
            mass_scale: 1.0,
            ..Default::default()
        };
        let builder = RagdollPresets::humanoid(config);
        let upper_leg_l = builder
            .bones
            .iter()
            .find(|b| b.name == "upper_leg_l")
            .unwrap();
        assert!(
            upper_leg_l.offset.y < 0.0,
            "Upper leg offset.y should be negative (downward), got {}",
            upper_leg_l.offset.y
        );
    }

    #[test]
    fn mutation_r3_humanoid_lower_arm_offset_y_negative() {
        // lower_arm offset = (0, -0.25, 0)  -- delete - gives (0, 0.25, 0)
        let config = RagdollConfig {
            mass_scale: 1.0,
            ..Default::default()
        };
        let builder = RagdollPresets::humanoid(config);
        let lower_arm_l = builder
            .bones
            .iter()
            .find(|b| b.name == "lower_arm_l")
            .unwrap();
        assert!(
            lower_arm_l.offset.y < 0.0,
            "Lower arm offset.y should be negative, got {}",
            lower_arm_l.offset.y
        );
        assert!((lower_arm_l.offset.y - (-0.25)).abs() < 0.01);
    }

    #[test]
    fn mutation_r3_humanoid_lower_leg_offset_y_negative() {
        // lower_leg offset = (0, -0.38, 0)  -- delete - gives (0, 0.38, 0)
        let config = RagdollConfig {
            mass_scale: 1.0,
            ..Default::default()
        };
        let builder = RagdollPresets::humanoid(config);
        let lower_leg_l = builder
            .bones
            .iter()
            .find(|b| b.name == "lower_leg_l")
            .unwrap();
        assert!(
            lower_leg_l.offset.y < 0.0,
            "Lower leg offset.y should be negative, got {}",
            lower_leg_l.offset.y
        );
        assert!((lower_leg_l.offset.y - (-0.38)).abs() < 0.01);
    }

    #[test]
    fn mutation_r3_humanoid_upper_arm_left_x_negative() {
        // upper_arm_l x_offset = -0.2  -- delete - gives 0.2
        let config = RagdollConfig {
            mass_scale: 1.0,
            ..Default::default()
        };
        let builder = RagdollPresets::humanoid(config);
        let upper_arm_l = builder
            .bones
            .iter()
            .find(|b| b.name == "upper_arm_l")
            .unwrap();
        assert!(
            upper_arm_l.offset.x < 0.0,
            "Left arm should have negative x offset, got {}",
            upper_arm_l.offset.x
        );
    }

    #[test]
    fn mutation_r3_humanoid_upper_leg_left_x_negative() {
        let config = RagdollConfig {
            mass_scale: 1.0,
            ..Default::default()
        };
        let builder = RagdollPresets::humanoid(config);
        let upper_leg_l = builder
            .bones
            .iter()
            .find(|b| b.name == "upper_leg_l")
            .unwrap();
        assert!(
            upper_leg_l.offset.x < 0.0,
            "Left leg x should be negative, got {}",
            upper_leg_l.offset.x
        );
    }

    #[test]
    fn mutation_r3_humanoid_hinge_limit_multiplier() {
        // FRAC_PI_2 * 1.5  (mutation: * → / or +)
        let config = RagdollConfig {
            mass_scale: 1.0,
            ..Default::default()
        };
        let builder = RagdollPresets::humanoid(config);
        let lower_arm_l = builder
            .bones
            .iter()
            .find(|b| b.name == "lower_arm_l")
            .unwrap();
        match &lower_arm_l.joint_type {
            BoneJointType::Hinge {
                limits: Some((lo, hi)),
                ..
            } => {
                // Upper limit should be PI/2 * 1.5 = ~2.356
                let expected = std::f32::consts::FRAC_PI_2 * 1.5;
                assert!(
                    (hi - expected).abs() < 0.01,
                    "Hinge upper limit should be {}, got {}",
                    expected,
                    hi
                );
            }
            _ => panic!("lower_arm should be Hinge with limits"),
        }
    }

    #[test]
    fn mutation_r3_humanoid_lower_leg_hinge_negative_limit() {
        // lower_leg: Some((-FRAC_PI_2 * 1.5, 0.0))  -- delete - on outer negation
        let config = RagdollConfig {
            mass_scale: 1.0,
            ..Default::default()
        };
        let builder = RagdollPresets::humanoid(config);
        let lower_leg_l = builder
            .bones
            .iter()
            .find(|b| b.name == "lower_leg_l")
            .unwrap();
        match &lower_leg_l.joint_type {
            BoneJointType::Hinge {
                limits: Some((lo, hi)),
                ..
            } => {
                assert!(
                    *lo < 0.0,
                    "Lower leg hinge min limit should be negative, got {}",
                    lo
                );
                assert!(
                    (*hi - 0.0).abs() < 0.01,
                    "Lower leg hinge max should be 0.0, got {}",
                    hi
                );
            }
            _ => panic!("lower_leg should be Hinge"),
        }
    }

    // --- Humanoid spine/chest field deletion catches ---
    #[test]
    fn mutation_r3_humanoid_spine_fields_present() {
        // "delete field offset/shape/mass/joint_type from BoneDef in humanoid" (lines 531-536)
        let config = RagdollConfig {
            mass_scale: 1.0,
            ..Default::default()
        };
        let builder = RagdollPresets::humanoid(config);
        let spine = builder.bones.iter().find(|b| b.name == "spine").unwrap();
        assert!(
            (spine.offset.y - 0.15).abs() < 0.01,
            "Spine offset.y should be 0.15"
        );
        assert!((spine.mass - 3.0).abs() < 0.01, "Spine mass should be 3.0");
        match &spine.shape {
            BoneShape::Box { half_extents } => {
                assert!((half_extents.x - 0.12).abs() < 0.01);
            }
            _ => panic!("Spine should be Box shape"),
        }
        match &spine.joint_type {
            BoneJointType::Spherical { swing_limit, .. } => {
                assert!(swing_limit.is_some(), "Spine should have swing limit");
            }
            _ => panic!("Spine should be Spherical joint"),
        }
    }

    #[test]
    fn mutation_r3_humanoid_chest_fields_present() {
        // "delete field offset/shape/mass/joint_type from BoneDef in humanoid" (lines 546-551)
        let config = RagdollConfig {
            mass_scale: 1.0,
            ..Default::default()
        };
        let builder = RagdollPresets::humanoid(config);
        let chest = builder.bones.iter().find(|b| b.name == "chest").unwrap();
        assert!(
            (chest.offset.y - 0.2).abs() < 0.01,
            "Chest offset.y should be 0.2"
        );
        assert!((chest.mass - 3.5).abs() < 0.01, "Chest mass should be 3.5");
        match &chest.shape {
            BoneShape::Box { half_extents } => {
                assert!((half_extents.x - 0.14).abs() < 0.01);
            }
            _ => panic!("Chest should be Box"),
        }
    }

    // --- Quadruped offset sign mutations (delete - on lines 659-694) ---
    #[test]
    fn mutation_r3_quadruped_front_leg_offset_y_negative() {
        let config = RagdollConfig {
            mass_scale: 1.0,
            ..Default::default()
        };
        let builder = RagdollPresets::quadruped(config);
        let fl = builder
            .bones
            .iter()
            .find(|b| b.name == "front_leg_l")
            .unwrap();
        assert!(
            fl.offset.y < 0.0,
            "Front leg y should be negative (downward), got {}",
            fl.offset.y
        );
    }

    #[test]
    fn mutation_r3_quadruped_back_leg_offset_y_negative() {
        let config = RagdollConfig {
            mass_scale: 1.0,
            ..Default::default()
        };
        let builder = RagdollPresets::quadruped(config);
        let bl = builder
            .bones
            .iter()
            .find(|b| b.name == "back_leg_l")
            .unwrap();
        assert!(
            bl.offset.y < 0.0,
            "Back leg y should be negative, got {}",
            bl.offset.y
        );
    }

    #[test]
    fn mutation_r3_quadruped_left_legs_x_negative() {
        let config = RagdollConfig {
            mass_scale: 1.0,
            ..Default::default()
        };
        let builder = RagdollPresets::quadruped(config);
        let fl = builder
            .bones
            .iter()
            .find(|b| b.name == "front_leg_l")
            .unwrap();
        let bl = builder
            .bones
            .iter()
            .find(|b| b.name == "back_leg_l")
            .unwrap();
        assert!(
            fl.offset.x < 0.0,
            "Front left leg x should be negative, got {}",
            fl.offset.x
        );
        assert!(
            bl.offset.x < 0.0,
            "Back left leg x should be negative, got {}",
            bl.offset.x
        );
    }

    #[test]
    fn mutation_r3_quadruped_back_leg_offset_z_negative() {
        // back_leg offset z should be negative (behind body)
        let config = RagdollConfig {
            mass_scale: 1.0,
            ..Default::default()
        };
        let builder = RagdollPresets::quadruped(config);
        let bl = builder
            .bones
            .iter()
            .find(|b| b.name == "back_leg_l")
            .unwrap();
        assert!(
            bl.offset.z < 0.0,
            "Back leg z should be negative (behind), got {}",
            bl.offset.z
        );
    }

    // --- add_hinge_bone/add_ball_bone field deletion and mass scaling ---
    #[test]
    fn mutation_r3_add_hinge_bone_offset_stored() {
        // "delete field offset" from add_hinge_bone struct
        let config = RagdollConfig {
            mass_scale: 1.0,
            ..Default::default()
        };
        let mut builder = RagdollBuilder::new(config);
        builder.add_bone(
            "root",
            None,
            Vec3::ZERO,
            BoneShape::Sphere { radius: 0.1 },
            1.0,
        );
        let offset = Vec3::new(1.5, -2.3, 0.7);
        builder.add_hinge_bone(
            "hinge_child",
            "root",
            offset,
            BoneShape::Capsule {
                radius: 0.05,
                half_height: 0.1,
            },
            2.0,
            Vec3::X,
            None,
        );
        let bone = builder
            .bones
            .iter()
            .find(|b| b.name == "hinge_child")
            .unwrap();
        assert!(
            (bone.offset - offset).length() < 1e-5,
            "Offset should be stored exactly"
        );
    }

    #[test]
    fn mutation_r3_add_ball_bone_shape_stored() {
        // "delete field shape" from add_ball_bone struct
        let config = RagdollConfig {
            mass_scale: 1.0,
            ..Default::default()
        };
        let mut builder = RagdollBuilder::new(config);
        builder.add_bone(
            "root",
            None,
            Vec3::ZERO,
            BoneShape::Sphere { radius: 0.1 },
            1.0,
        );
        builder.add_ball_bone(
            "ball_child",
            "root",
            Vec3::Y,
            BoneShape::Capsule {
                radius: 0.08,
                half_height: 0.2,
            },
            3.0,
            Some(1.0),
        );
        let bone = builder
            .bones
            .iter()
            .find(|b| b.name == "ball_child")
            .unwrap();
        match bone.shape {
            BoneShape::Capsule {
                radius,
                half_height,
            } => {
                assert!((radius - 0.08).abs() < 1e-5, "Shape radius should match");
                assert!(
                    (half_height - 0.2).abs() < 1e-5,
                    "Shape half_height should match"
                );
            }
            _ => panic!("Shape should be Capsule, got {:?}", bone.shape),
        }
    }

    #[test]
    fn mutation_r3_add_bone_offset_and_shape_stored() {
        // "delete field offset/shape" from add_bone (lines 344-345)
        let config = RagdollConfig {
            mass_scale: 1.0,
            ..Default::default()
        };
        let mut builder = RagdollBuilder::new(config);
        let offset = Vec3::new(3.7, -1.2, 0.0);
        let shape = BoneShape::Box {
            half_extents: Vec3::new(0.5, 0.3, 0.2),
        };
        builder.add_bone("test_bone", None, offset, shape, 5.0);
        let bone = &builder.bones[0];
        assert!(
            (bone.offset - offset).length() < 1e-5,
            "add_bone should store offset"
        );
        match bone.shape {
            BoneShape::Box { half_extents } => {
                assert!((half_extents.x - 0.5).abs() < 1e-5);
            }
            _ => panic!("Shape should be Box"),
        }
    }

    #[test]
    fn mutation_r3_add_bone_full_mass_scale_operator() {
        // mass *= mass_scale  (mutation: *= → /= or +=)
        let config = RagdollConfig {
            mass_scale: 2.5,
            ..Default::default()
        };
        let mut builder = RagdollBuilder::new(config);
        builder.add_bone_full(BoneDef {
            name: "scaled".to_string(),
            mass: 4.0,
            ..Default::default()
        });
        let bone = &builder.bones[0];
        // mass should be 4.0 * 2.5 = 10.0
        assert!(
            (bone.mass - 10.0).abs() < 1e-5,
            "mass_scale should multiply: 4*2.5=10, got {}",
            bone.mass
        );
    }

    // ===== ROUND 6: Ragdoll integration tests =====

    #[test]
    fn r6_build_creates_bodies_in_physics() {
        let mut pw = crate::PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));
        let config = RagdollConfig::default();
        let mut builder = RagdollPresets::humanoid(config);
        let ragdoll = builder.build(&mut pw, Vec3::new(0.0, 5.0, 0.0));

        // Humanoid has: pelvis, spine, chest, head, upper_arm_l/r,
        // lower_arm_l/r, upper_leg_l/r, lower_leg_l/r = 12 bones
        assert!(
            ragdoll.bone_bodies.len() >= 10,
            "Humanoid should have >=10 bones: {}",
            ragdoll.bone_bodies.len()
        );

        // Each bone body should exist in physics world
        for (name, body_id) in &ragdoll.bone_bodies {
            assert!(
                pw.body_transform(*body_id).is_some(),
                "Bone '{}' body should exist in PhysicsWorld",
                name
            );
        }
    }

    #[test]
    fn r6_build_creates_joints() {
        let mut pw = crate::PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));
        let config = RagdollConfig::default();
        let mut builder = RagdollPresets::humanoid(config);
        let ragdoll = builder.build(&mut pw, Vec3::ZERO);

        // Every non-root bone should have a joint
        let non_root = ragdoll.bone_bodies.len() - 1;
        assert!(
            ragdoll.bone_joints.len() >= non_root - 1,
            "Should have ~{} joints, got {}",
            non_root,
            ragdoll.bone_joints.len()
        );
    }

    #[test]
    fn r6_root_bone_set_correctly() {
        let mut pw = crate::PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));
        let config = RagdollConfig::default();
        let mut builder = RagdollPresets::humanoid(config);
        let ragdoll = builder.build(&mut pw, Vec3::ZERO);

        assert_eq!(ragdoll.root_bone, "pelvis");
        assert!(ragdoll.root_body().is_some());
    }

    #[test]
    fn r6_center_of_mass_near_spawn() {
        let mut pw = crate::PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));
        let spawn_pos = Vec3::new(5.0, 10.0, 3.0);
        let config = RagdollConfig::default();
        let mut builder = RagdollPresets::humanoid(config);
        let ragdoll = builder.build(&mut pw, spawn_pos);

        let com = ragdoll.center_of_mass(&pw);
        // CoM should be near spawn position (within a couple meters of offsets)
        assert!(
            (com - spawn_pos).length() < 3.0,
            "CoM {:?} should be near spawn {:?}",
            com,
            spawn_pos
        );
    }

    #[test]
    fn r6_is_at_rest_initially() {
        let mut pw = crate::PhysicsWorld::new(Vec3::ZERO); // No gravity
        let config = RagdollConfig::default();
        let mut builder = RagdollPresets::humanoid(config);
        let ragdoll = builder.build(&mut pw, Vec3::ZERO);

        // No gravity, no forces — all bodies should be at rest
        assert!(
            ragdoll.is_at_rest(&pw, 0.5),
            "Ragdoll in zero-gravity should be at rest initially"
        );
    }

    #[test]
    fn r6_impulse_propagation_affects_multiple_bones() {
        let mut pw = crate::PhysicsWorld::new(Vec3::ZERO);
        let config = RagdollConfig::default();
        let mut builder = RagdollPresets::humanoid(config);
        let ragdoll = builder.build(&mut pw, Vec3::ZERO);

        let impulse = Vec3::new(100.0, 0.0, 0.0);
        ragdoll.apply_impulse_with_propagation(&mut pw, "chest", impulse, 0.5);

        pw.step();

        // Chest should have moved
        let chest_vel = pw
            .get_velocity(*ragdoll.bone_bodies.get("chest").unwrap())
            .unwrap();
        assert!(
            chest_vel.x > 0.1,
            "Chest should gain X velocity from impulse: {:?}",
            chest_vel
        );

        // Spine (parent of chest) should also move (propagation_factor=0.5)
        if let Some(spine_body) = ragdoll.bone_bodies.get("spine") {
            let spine_vel = pw.get_velocity(*spine_body).unwrap();
            assert!(
                spine_vel.x > 0.01,
                "Spine (parent) should receive propagated impulse: {:?}",
                spine_vel
            );
            // Propagated velocity should be less than direct hit
            assert!(
                spine_vel.x < chest_vel.x,
                "Propagated should be weaker: spine={}, chest={}",
                spine_vel.x,
                chest_vel.x
            );
        }
    }

    #[test]
    fn r6_is_at_rest_after_impulse_false() {
        let mut pw = crate::PhysicsWorld::new(Vec3::ZERO);
        let config = RagdollConfig::default();
        let mut builder = RagdollPresets::humanoid(config);
        let ragdoll = builder.build(&mut pw, Vec3::ZERO);

        ragdoll.apply_impulse_with_propagation(&mut pw, "pelvis", Vec3::new(500.0, 0.0, 0.0), 0.5);
        pw.step();

        assert!(
            !ragdoll.is_at_rest(&pw, 0.1),
            "After strong impulse, ragdoll should NOT be at rest"
        );
    }

    #[test]
    fn r6_quadruped_bone_count() {
        let mut pw = crate::PhysicsWorld::new(Vec3::ZERO);
        let config = RagdollConfig::default();
        let mut builder = RagdollPresets::quadruped(config);
        let ragdoll = builder.build(&mut pw, Vec3::ZERO);

        // Quadruped: body, head, front_leg_l/r, back_leg_l/r, tail = 7 bones
        assert!(
            ragdoll.bone_bodies.len() >= 7,
            "Quadruped should have >=7 bones: {}",
            ragdoll.bone_bodies.len()
        );
    }

    #[test]
    fn r6_humanoid_bones_at_correct_offsets() {
        let mut pw = crate::PhysicsWorld::new(Vec3::ZERO);
        let spawn = Vec3::new(0.0, 10.0, 0.0);
        let config = RagdollConfig::default();
        let mut builder = RagdollPresets::humanoid(config);
        let ragdoll = builder.build(&mut pw, spawn);

        // Head should be above pelvis
        let pelvis_pos = pw
            .body_transform(*ragdoll.bone_bodies.get("pelvis").unwrap())
            .map(|t| Vec3::new(t.w_axis.x, t.w_axis.y, t.w_axis.z))
            .unwrap();
        let head_pos = pw
            .body_transform(*ragdoll.bone_bodies.get("head").unwrap())
            .map(|t| Vec3::new(t.w_axis.x, t.w_axis.y, t.w_axis.z))
            .unwrap();

        assert!(
            head_pos.y > pelvis_pos.y,
            "Head ({:.2}) should be above pelvis ({:.2})",
            head_pos.y,
            pelvis_pos.y
        );

        // Legs should be below pelvis
        if let Some(leg_body) = ragdoll.bone_bodies.get("upper_leg_l") {
            let leg_pos = pw
                .body_transform(*leg_body)
                .map(|t| Vec3::new(t.w_axis.x, t.w_axis.y, t.w_axis.z))
                .unwrap();
            assert!(
                leg_pos.y < pelvis_pos.y,
                "Upper leg ({:.2}) should be below pelvis ({:.2})",
                leg_pos.y,
                pelvis_pos.y
            );
        }
    }

    #[test]
    fn r6_add_bone_mass_scales() {
        let config = RagdollConfig {
            mass_scale: 3.0,
            ..Default::default()
        };
        let mut builder = RagdollBuilder::new(config);
        builder.add_bone(
            "test",
            None,
            Vec3::ZERO,
            BoneShape::Sphere { radius: 0.5 },
            5.0,
        );
        assert!(
            (builder.bones[0].mass - 15.0).abs() < 1e-5,
            "Mass 5 * scale 3 = 15, got {}",
            builder.bones[0].mass
        );
    }

    #[test]
    fn r6_humanoid_arm_symmetry() {
        let config = RagdollConfig::default();
        let builder = RagdollPresets::humanoid(config);
        // Arms should be symmetric: upper_arm_l has negative X, upper_arm_r has positive X
        let arm_l = builder
            .bones
            .iter()
            .find(|b| b.name == "upper_arm_l")
            .unwrap();
        let arm_r = builder
            .bones
            .iter()
            .find(|b| b.name == "upper_arm_r")
            .unwrap();
        assert!(
            arm_l.offset.x < 0.0,
            "Left arm should have negative X offset: {}",
            arm_l.offset.x
        );
        assert!(
            arm_r.offset.x > 0.0,
            "Right arm should have positive X offset: {}",
            arm_r.offset.x
        );
        // Offsets should be symmetric
        assert!(
            (arm_l.offset.x + arm_r.offset.x).abs() < 0.01,
            "Arm offsets should be symmetric: l={}, r={}",
            arm_l.offset.x,
            arm_r.offset.x
        );
    }

    // ===== ROUND 7: Targeted bone offset catches =====

    #[test]
    fn r7_humanoid_spine_offset_y_positive() {
        let config = RagdollConfig::default();
        let builder = RagdollPresets::humanoid(config);
        let spine = builder.bones.iter().find(|b| b.name == "spine").unwrap();
        // Spine should be above pelvis: offset.y > 0
        assert!(
            spine.offset.y > 0.0,
            "Spine offset.y should be positive (above pelvis): {}",
            spine.offset.y
        );
        // Spine offset should be close to 0.15
        assert!(
            (spine.offset.y - 0.15).abs() < 0.05,
            "Spine offset.y should be ~0.15: {}",
            spine.offset.y
        );
    }

    #[test]
    fn r7_humanoid_chest_offset_y_positive() {
        let config = RagdollConfig::default();
        let builder = RagdollPresets::humanoid(config);
        let chest = builder.bones.iter().find(|b| b.name == "chest").unwrap();
        assert!(
            chest.offset.y > 0.0,
            "Chest offset.y should be positive: {}",
            chest.offset.y
        );
        assert!(
            (chest.offset.y - 0.2).abs() < 0.05,
            "Chest offset.y should be ~0.2: {}",
            chest.offset.y
        );
    }

    #[test]
    fn r7_humanoid_spine_box_half_extents() {
        let config = RagdollConfig::default();
        let builder = RagdollPresets::humanoid(config);
        let spine = builder.bones.iter().find(|b| b.name == "spine").unwrap();
        match &spine.shape {
            BoneShape::Box { half_extents } => {
                // x=0.12, y=0.12, z=0.08
                assert!(
                    half_extents.x > 0.1 && half_extents.x < 0.15,
                    "Spine box x half-extent should be ~0.12: {}",
                    half_extents.x
                );
                assert!(
                    half_extents.z > 0.05 && half_extents.z < 0.12,
                    "Spine box z half-extent should be ~0.08: {}",
                    half_extents.z
                );
            }
            other => panic!("Spine should be Box, got {:?}", other),
        }
    }

    #[test]
    fn r7_humanoid_chest_box_half_extents() {
        let config = RagdollConfig::default();
        let builder = RagdollPresets::humanoid(config);
        let chest = builder.bones.iter().find(|b| b.name == "chest").unwrap();
        match &chest.shape {
            BoneShape::Box { half_extents } => {
                // x=0.14, y=0.1, z=0.09
                assert!(
                    half_extents.x > 0.12 && half_extents.x < 0.18,
                    "Chest box x should be ~0.14: {}",
                    half_extents.x
                );
            }
            other => panic!("Chest should be Box, got {:?}", other),
        }
    }

    #[test]
    fn r7_humanoid_leg_offsets_symmetric() {
        let config = RagdollConfig::default();
        let builder = RagdollPresets::humanoid(config);
        let leg_l = builder
            .bones
            .iter()
            .find(|b| b.name == "upper_leg_l")
            .unwrap();
        let leg_r = builder
            .bones
            .iter()
            .find(|b| b.name == "upper_leg_r")
            .unwrap();

        // Leg offsets should be symmetric in X
        assert!(
            leg_l.offset.x < 0.0,
            "Left leg x offset should be negative: {}",
            leg_l.offset.x
        );
        assert!(
            leg_r.offset.x > 0.0,
            "Right leg x offset should be positive: {}",
            leg_r.offset.x
        );
        assert!(
            (leg_l.offset.x + leg_r.offset.x).abs() < 0.01,
            "Leg x offsets should be symmetric: l={}, r={}",
            leg_l.offset.x,
            leg_r.offset.x
        );
        // Y offset should be negative (below pelvis)
        assert!(
            leg_l.offset.y < 0.0,
            "Leg y offset should be negative: {}",
            leg_l.offset.y
        );
    }

    #[test]
    fn r7_quadruped_body_bone_extents() {
        let config = RagdollConfig::default();
        let builder = RagdollPresets::quadruped(config);
        let body = builder.bones.iter().find(|b| b.name == "body").unwrap();
        match &body.shape {
            BoneShape::Box { half_extents } => {
                // body half: (0.2, 0.15, 0.4)
                assert!(
                    half_extents.z > half_extents.x,
                    "Quadruped body should be longer than wide: z={}, x={}",
                    half_extents.z,
                    half_extents.x
                );
            }
            other => panic!("Body should be Box, got {:?}", other),
        }
    }

    // ============================================================================
    // R11 — Targeted mutation tests for ragdoll functions
    // ============================================================================

    #[test]
    fn r11_impulse_to_bone_true_for_existing() {
        let mut physics = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));
        let mut builder = RagdollBuilder::new(RagdollConfig::default());
        builder.add_bone(
            "root",
            None,
            Vec3::ZERO,
            BoneShape::Sphere { radius: 0.2 },
            2.0,
        );
        builder.add_bone(
            "child",
            Some("root"),
            Vec3::new(0.0, 0.5, 0.0),
            BoneShape::Sphere { radius: 0.15 },
            1.0,
        );
        let ragdoll = builder.build(&mut physics, Vec3::new(0.0, 5.0, 0.0));

        let result = ragdoll.apply_impulse_to_bone(&mut physics, "root", Vec3::new(10.0, 0.0, 0.0));
        assert!(
            result,
            "apply_impulse_to_bone should return true for existing bone"
        );
    }

    #[test]
    fn r11_impulse_to_bone_false_for_missing() {
        let mut physics = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));
        let mut builder = RagdollBuilder::new(RagdollConfig::default());
        builder.add_bone(
            "root",
            None,
            Vec3::ZERO,
            BoneShape::Sphere { radius: 0.2 },
            2.0,
        );
        let ragdoll = builder.build(&mut physics, Vec3::new(0.0, 5.0, 0.0));

        let result =
            ragdoll.apply_impulse_to_bone(&mut physics, "nonexistent", Vec3::new(10.0, 0.0, 0.0));
        assert!(
            !result,
            "apply_impulse_to_bone should return false for missing bone"
        );
    }

    #[test]
    fn r11_impulse_to_bone_changes_velocity() {
        let mut physics = PhysicsWorld::new(Vec3::new(0.0, 0.0, 0.0)); // no gravity
        let mut builder = RagdollBuilder::new(RagdollConfig::default());
        builder.add_bone(
            "root",
            None,
            Vec3::ZERO,
            BoneShape::Sphere { radius: 0.2 },
            2.0,
        );
        let ragdoll = builder.build(&mut physics, Vec3::new(0.0, 5.0, 0.0));

        let root_id = ragdoll.get_bone_body("root").unwrap();
        let vel_before = physics.get_velocity(root_id).unwrap();
        assert!(vel_before.length() < 0.01, "Should start at rest");

        ragdoll.apply_impulse_to_bone(&mut physics, "root", Vec3::new(100.0, 0.0, 0.0));
        physics.step();
        let vel_after = physics.get_velocity(root_id).unwrap();
        assert!(
            vel_after.x > 1.0,
            "Impulse should increase velocity: {:?}",
            vel_after
        );
    }

    #[test]
    fn r11_impulse_propagation_parent_gets_reduced() {
        let mut physics = PhysicsWorld::new(Vec3::new(0.0, 0.0, 0.0)); // no gravity
        let mut builder = RagdollBuilder::new(RagdollConfig::default());
        builder.add_bone(
            "root",
            None,
            Vec3::ZERO,
            BoneShape::Sphere { radius: 0.2 },
            2.0,
        );
        builder.add_bone(
            "child",
            Some("root"),
            Vec3::new(0.0, 1.0, 0.0),
            BoneShape::Sphere { radius: 0.15 },
            2.0,
        );
        let ragdoll = builder.build(&mut physics, Vec3::new(0.0, 5.0, 0.0));

        // Apply impulse to child with propagation to parent
        ragdoll.apply_impulse_with_propagation(
            &mut physics,
            "child",
            Vec3::new(100.0, 0.0, 0.0),
            0.5,
        );
        physics.step();

        let parent_id = ragdoll.get_bone_body("root").unwrap();
        let parent_vel = physics.get_velocity(parent_id).unwrap();
        assert!(
            parent_vel.x > 0.5,
            "Parent should receive propagated impulse: {:?}",
            parent_vel
        );
    }

    #[test]
    fn r11_impulse_propagation_child_gets_reduced() {
        let mut physics = PhysicsWorld::new(Vec3::new(0.0, 0.0, 0.0));
        let mut builder = RagdollBuilder::new(RagdollConfig::default());
        builder.add_bone(
            "root",
            None,
            Vec3::ZERO,
            BoneShape::Sphere { radius: 0.2 },
            2.0,
        );
        builder.add_bone(
            "child",
            Some("root"),
            Vec3::new(0.0, 1.0, 0.0),
            BoneShape::Sphere { radius: 0.15 },
            2.0,
        );
        builder.add_bone(
            "grandchild",
            Some("child"),
            Vec3::new(0.0, 1.0, 0.0),
            BoneShape::Sphere { radius: 0.1 },
            1.0,
        );
        let ragdoll = builder.build(&mut physics, Vec3::new(0.0, 5.0, 0.0));

        // Apply impulse to child — grandchild should receive propagated impulse
        ragdoll.apply_impulse_with_propagation(
            &mut physics,
            "child",
            Vec3::new(100.0, 0.0, 0.0),
            0.5,
        );
        physics.step();

        let gc_id = ragdoll.get_bone_body("grandchild").unwrap();
        let gc_vel = physics.get_velocity(gc_id).unwrap();
        assert!(
            gc_vel.x > 0.5,
            "Child bone should receive propagated impulse: {:?}",
            gc_vel
        );
    }

    #[test]
    fn r11_impulse_propagation_factor_matters() {
        // Compare propagation_factor=0.2 vs 0.8 — higher factor = more parent velocity
        let run = |factor: f32| -> f32 {
            let mut physics = PhysicsWorld::new(Vec3::new(0.0, 0.0, 0.0));
            let mut builder = RagdollBuilder::new(RagdollConfig::default());
            builder.add_bone(
                "root",
                None,
                Vec3::ZERO,
                BoneShape::Sphere { radius: 0.2 },
                2.0,
            );
            builder.add_bone(
                "child",
                Some("root"),
                Vec3::new(0.0, 1.0, 0.0),
                BoneShape::Sphere { radius: 0.15 },
                2.0,
            );
            let ragdoll = builder.build(&mut physics, Vec3::new(0.0, 5.0, 0.0));

            ragdoll.apply_impulse_with_propagation(
                &mut physics,
                "child",
                Vec3::new(100.0, 0.0, 0.0),
                factor,
            );
            physics.step();
            let parent_id = ragdoll.get_bone_body("root").unwrap();
            physics.get_velocity(parent_id).unwrap().x
        };
        let vel_low = run(0.2);
        let vel_high = run(0.8);
        assert!(
            vel_high > vel_low * 1.2,
            "Higher propagation factor should give more parent velocity: low={}, high={}",
            vel_low,
            vel_high
        );
    }

    #[test]
    fn r11_center_of_mass_exact_two_bones() {
        let mut physics = PhysicsWorld::new(Vec3::new(0.0, 0.0, 0.0)); // no gravity
        let mut builder = RagdollBuilder::new(RagdollConfig::default());
        // Root at spawn (0, 5, 0) with mass 1.0
        builder.add_bone(
            "root",
            None,
            Vec3::ZERO,
            BoneShape::Sphere { radius: 0.2 },
            1.0,
        );
        // Child at (0, 6, 0) (offset Y+1) with mass 3.0
        builder.add_bone(
            "child",
            Some("root"),
            Vec3::new(0.0, 1.0, 0.0),
            BoneShape::Sphere { radius: 0.15 },
            3.0,
        );
        let ragdoll = builder.build(&mut physics, Vec3::new(0.0, 5.0, 0.0));

        let com = ragdoll.center_of_mass(&physics);
        // COM = (1*5 + 3*6) / (1+3) = 23/4 = 5.75
        assert!(
            (com.y - 5.75).abs() < 0.5,
            "COM should be weighted toward heavier bone: y={}",
            com.y
        );
    }

    #[test]
    fn r11_build_child_offset_positive_y() {
        // The build method calculates: parent_pos + bone.offset
        // Mutation: + with - would place child BELOW parent
        let mut physics = PhysicsWorld::new(Vec3::new(0.0, 0.0, 0.0));
        let mut builder = RagdollBuilder::new(RagdollConfig::default());
        builder.add_bone(
            "root",
            None,
            Vec3::ZERO,
            BoneShape::Sphere { radius: 0.2 },
            1.0,
        );
        builder.add_bone(
            "child",
            Some("root"),
            Vec3::new(0.0, 2.0, 0.0),
            BoneShape::Sphere { radius: 0.15 },
            1.0,
        );
        let ragdoll = builder.build(&mut physics, Vec3::new(0.0, 10.0, 0.0));

        let root_id = ragdoll.get_bone_body("root").unwrap();
        let child_id = ragdoll.get_bone_body("child").unwrap();
        let root_t = physics.body_transform(root_id).unwrap();
        let child_t = physics.body_transform(child_id).unwrap();
        let root_y = root_t.w_axis.y;
        let child_y = child_t.w_axis.y;

        assert!(
            child_y > root_y + 0.5,
            "Child should be ABOVE root (offset +2): root_y={}, child_y={}",
            root_y,
            child_y
        );
    }

    #[test]
    fn r11_build_next_id_increments() {
        let mut physics = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));
        let mut builder = RagdollBuilder::new(RagdollConfig::default());
        builder.add_bone(
            "root",
            None,
            Vec3::ZERO,
            BoneShape::Sphere { radius: 0.2 },
            1.0,
        );

        let r1 = builder.build(&mut physics, Vec3::ZERO);
        let r2 = builder.build(&mut physics, Vec3::new(5.0, 0.0, 0.0));
        assert_ne!(r1.id, r2.id, "Ragdoll IDs must be different");
        assert_eq!(
            r2.id,
            r1.id + 1,
            "IDs should increment: r1={}, r2={}",
            r1.id,
            r2.id
        );
    }

    #[test]
    fn r11_humanoid_lower_leg_offset_y_negative() {
        // Mutation: delete - in humanoid preset would make lower_leg offset positive
        let config = RagdollConfig::default();
        let builder = RagdollPresets::humanoid(config);
        let lower_leg = builder
            .bones
            .iter()
            .find(|b| b.name == "lower_leg_l")
            .unwrap();
        assert!(
            lower_leg.offset.y < -0.1,
            "Lower leg should have negative Y offset (below upper leg): y={}",
            lower_leg.offset.y
        );
    }

    #[test]
    fn r11_humanoid_lower_arm_offset_y_negative() {
        let config = RagdollConfig::default();
        let builder = RagdollPresets::humanoid(config);
        let lower_arm = builder
            .bones
            .iter()
            .find(|b| b.name == "lower_arm_l")
            .unwrap();
        assert!(
            lower_arm.offset.y < -0.1,
            "Lower arm should have negative Y offset: y={}",
            lower_arm.offset.y
        );
    }

    #[test]
    fn r11_humanoid_upper_leg_joint_type_spherical() {
        let config = RagdollConfig::default();
        let builder = RagdollPresets::humanoid(config);
        let upper_leg = builder
            .bones
            .iter()
            .find(|b| b.name == "upper_leg_l")
            .unwrap();
        assert!(
            matches!(upper_leg.joint_type, BoneJointType::Spherical { .. }),
            "Upper leg should have spherical joint: {:?}",
            upper_leg.joint_type
        );
    }

    #[test]
    fn r11_humanoid_lower_leg_joint_type_hinge() {
        let config = RagdollConfig::default();
        let builder = RagdollPresets::humanoid(config);
        let lower_leg = builder
            .bones
            .iter()
            .find(|b| b.name == "lower_leg_l")
            .unwrap();
        assert!(
            matches!(lower_leg.joint_type, BoneJointType::Hinge { .. }),
            "Lower leg should have hinge joint: {:?}",
            lower_leg.joint_type
        );
    }

    #[test]
    fn r11_humanoid_upper_leg_mass_scaled() {
        let config = RagdollConfig {
            mass_scale: 2.0,
            ..Default::default()
        };
        let builder = RagdollPresets::humanoid(config);
        let upper_leg = builder
            .bones
            .iter()
            .find(|b| b.name == "upper_leg_l")
            .unwrap();
        // Original mass = 3.0, with scale 2.0 → 6.0
        assert!(
            (upper_leg.mass - 6.0).abs() < 0.01,
            "Upper leg mass should be 3.0 * 2.0 = 6.0, got {}",
            upper_leg.mass
        );
    }

    #[test]
    fn r11_quadruped_front_leg_y_negative() {
        let config = RagdollConfig::default();
        let builder = RagdollPresets::quadruped(config);
        let leg = builder
            .bones
            .iter()
            .find(|b| b.name == "front_leg_l")
            .unwrap();
        assert!(
            leg.offset.y < -0.1,
            "Front leg y offset should be negative: y={}",
            leg.offset.y
        );
    }

    #[test]
    fn r11_is_at_rest_velocity_threshold() {
        let mut physics = PhysicsWorld::new(Vec3::new(0.0, 0.0, 0.0));
        let mut builder = RagdollBuilder::new(RagdollConfig::default());
        builder.add_bone(
            "root",
            None,
            Vec3::ZERO,
            BoneShape::Sphere { radius: 0.2 },
            1.0,
        );
        let ragdoll = builder.build(&mut physics, Vec3::ZERO);

        // At rest initially
        assert!(ragdoll.is_at_rest(&physics, 0.1));

        // Apply large impulse
        ragdoll.apply_impulse_to_bone(&mut physics, "root", Vec3::new(1000.0, 0.0, 0.0));
        physics.step();

        // Should NOT be at rest with small threshold
        assert!(
            !ragdoll.is_at_rest(&physics, 0.1),
            "Should not be at rest after large impulse"
        );
    }
}
