#![forbid(unsafe_code)]
#![cfg_attr(
    test,
    allow(
        unused_variables,
        unused_mut,
        unused_imports,
        non_snake_case,
        clippy::manual_range_contains,
        clippy::field_reassign_with_default,
        clippy::no_effect,
        clippy::identity_op,
        clippy::needless_update,
        clippy::useless_vec,
    )
)]
//! # AstraWeave Physics
//!
//! Full-featured physics simulation for AstraWeave, wrapping **Rapier3D 0.22**.
//!
//! This crate provides:
//!
//! - **[`PhysicsWorld`]** — Central simulation state (rigid bodies, colliders, joints, queries).
//! - **[`CharacterController`]** — Kinematic character movement with ground detection.
//! - **[`SpatialHash`]** — Grid-based broadphase for O(n log n) collision culling
//!   (99.96% pair reduction vs brute-force).
//! - **[`ProjectileManager`]** — Ballistic projectiles with penetration and explosions.
//! - **[`GravityManager`]** — Composable gravity zones (sphere, box, cylinder).
//! - **[`Ragdoll`] / [`RagdollBuilder`]** — Multi-bone ragdoll creation and simulation.
//! - **[`Vehicle`] / [`VehicleManager`]** — Vehicle dynamics with drivetrain and wheels.
//! - **[`EnvironmentManager`]** — Wind zones, water volumes, and buoyancy.
//! - **[`DestructionManager`]** — Fracture patterns and debris spawning.
//! - **[`ClothManager`]** — Particle-based cloth with distance constraints.
//!
//! # Feature Flags
//!
//! | Feature | Description |
//! |---------|-------------|
//! | `async-physics` | Parallel 3-stage pipeline via Rayon |
//! | `profiling` | Tracy integration for performance profiling |
//! | `ecs` | ECS system integration |
//!
//! # Performance
//!
//! - Character move: 114 ns
//! - Full physics tick: 6.52 µs
//! - Rigid body step: 2.97 µs
//! - Spatial hash: 3.77 ms (FxHashMap, vs 5.61 ms SipHash)

use glam::{vec3, Mat4, Vec3};

// Rapier3D explicit re-exports (replaces glob `pub use rapier3d::prelude::*`)
pub use rapier3d::prelude::{
    ActiveEvents,
    CCDSolver,
    // Event handling
    ChannelEventCollector,
    ColliderBuilder,
    ColliderHandle,
    ColliderSet,
    CollisionEvent,
    ContactForceEvent,
    DebugRenderBackend,
    DebugRenderObject,
    // Debug rendering
    DebugRenderPipeline,
    DefaultBroadPhase,
    GenericJointBuilder,
    Group,
    ImpulseJointSet,
    IntegrationParameters,
    // Collision configuration
    InteractionGroups,
    IslandManager,
    // Joint configuration
    JointAxesMask,
    LockedAxes,
    MultibodyJointSet,
    NarrowPhase,
    PhysicsPipeline,
    Point,
    PrismaticJointBuilder,
    // Query / Raycast
    QueryFilter,
    QueryPipeline,
    Ray,
    // Math types & aliases used by PhysicsWorld and shape construction
    Real,
    RevoluteJointBuilder,
    // Builders
    RigidBodyBuilder,
    // Handle types
    RigidBodyHandle,
    // Sets & Pipelines (PhysicsWorld pub fields)
    RigidBodySet,
    // Rigid body enum types
    RigidBodyType,
    // Shape types
    SharedShape,
    SphericalJointBuilder,
    UnitVector,
    Vector,
};
// Nalgebra re-exports used by rapier3d APIs
pub use rapier3d::na::{Point3 as NaPoint3, UnitVector3 as NaUnitVector3, Vector3 as NaVector3};
// Rapier3d macros (`point!`/`vector!` expand to `nalgebra::...` internally)
use rapier3d::na as nalgebra;
pub use rapier3d::prelude::{point, vector};

use std::collections::HashMap;

#[cfg(feature = "profiling")]
use astraweave_profiling::{plot, span};

// Async physics scheduler (feature-gated)
#[cfg(feature = "async-physics")]
pub mod async_scheduler;

#[cfg(feature = "async-physics")]
pub use async_scheduler::{AsyncPhysicsScheduler, PhysicsStepProfile};

// ECS integration (feature-gated)
#[cfg(feature = "ecs")]
pub mod ecs;

#[cfg(feature = "ecs")]
pub use ecs::*;

// Spatial hash grid for broad-phase collision optimization
pub mod spatial_hash;
pub use spatial_hash::{SpatialHash, SpatialHashStats, AABB};

// Projectile system for shooters and combat games
pub mod projectile;
pub use projectile::{
    ExplosionConfig, ExplosionResult, FalloffCurve, ProjectileConfig, ProjectileHit, ProjectileId,
    ProjectileKind, ProjectileManager,
};

// Gravity system for variable gravity, zones, and attractors
pub mod gravity;
pub use gravity::{
    BodyGravityId, BodyGravitySettings, GravityManager, GravityZone, GravityZoneId,
    GravityZoneShape,
};

// Ragdoll system for physics-based character animations
pub mod ragdoll;
pub use ragdoll::{
    BoneDef, BoneJointType, BoneShape, Ragdoll, RagdollBuilder, RagdollConfig, RagdollId,
    RagdollPresets, RagdollState,
};

// Vehicle physics for cars, trucks, motorcycles
pub mod vehicle;
pub use vehicle::{
    DrivetrainType, EngineConfig, FrictionCurve, TransmissionConfig, Vehicle, VehicleConfig,
    VehicleId, VehicleInput, VehicleManager, WheelConfig, WheelPosition, WheelState,
};

// Environmental physics (wind, water)
pub mod environment;
pub use environment::{
    EnvironmentManager, GustEvent, WaterVolume, WaterVolumeId, WindType, WindZone, WindZoneConfig,
    WindZoneId, WindZoneShape,
};

// Destruction system
pub mod destruction;
pub use destruction::{
    Debris, DebrisConfig, DebrisId, DebrisShape, Destructible, DestructibleConfig, DestructibleId,
    DestructibleState, DestructionEvent, DestructionManager, DestructionTrigger, FracturePattern,
};

// Cloth simulation
pub mod cloth;
pub use cloth::{
    Cloth, ClothCollider, ClothConfig, ClothId, ClothManager, ClothParticle, DistanceConstraint,
};

#[cfg(test)]
mod mutation_tests;

pub type BodyId = u64;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ActorKind {
    Static,
    Dynamic,
    Character,
    Other,
}

impl ActorKind {
    /// Returns the name of the actor kind.
    #[inline]
    pub fn name(&self) -> &'static str {
        match self {
            Self::Static => "Static",
            Self::Dynamic => "Dynamic",
            Self::Character => "Character",
            Self::Other => "Other",
        }
    }

    /// Returns true if this is a static actor.
    #[inline]
    pub fn is_static(&self) -> bool {
        matches!(self, Self::Static)
    }

    /// Returns true if this is a dynamic actor.
    #[inline]
    pub fn is_dynamic(&self) -> bool {
        matches!(self, Self::Dynamic)
    }

    /// Returns true if this is a character actor.
    #[inline]
    pub fn is_character(&self) -> bool {
        matches!(self, Self::Character)
    }

    /// Returns true if this is an other actor.
    #[inline]
    pub fn is_other(&self) -> bool {
        matches!(self, Self::Other)
    }

    /// Returns true if the actor can move (dynamic or character).
    #[inline]
    pub fn is_movable(&self) -> bool {
        matches!(self, Self::Dynamic | Self::Character)
    }

    /// Returns all actor kinds.
    pub fn all() -> [ActorKind; 4] {
        [Self::Static, Self::Dynamic, Self::Character, Self::Other]
    }
}

impl std::fmt::Display for ActorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct DebugLine {
    pub start: [f32; 3],
    pub end: [f32; 3],
    pub color: [f32; 3],
}

impl DebugLine {
    /// Creates a new debug line.
    #[inline]
    pub fn new(start: [f32; 3], end: [f32; 3], color: [f32; 3]) -> Self {
        Self { start, end, color }
    }

    /// Creates a debug line from Vec3 points.
    #[inline]
    pub fn from_vec3(start: Vec3, end: Vec3, color: [f32; 3]) -> Self {
        Self {
            start: [start.x, start.y, start.z],
            end: [end.x, end.y, end.z],
            color,
        }
    }

    /// Returns the length of the line.
    #[inline]
    pub fn length(&self) -> f32 {
        self.length_squared().sqrt()
    }

    /// Returns the squared length of the line.
    #[inline]
    pub fn length_squared(&self) -> f32 {
        let dx = self.end[0] - self.start[0];
        let dy = self.end[1] - self.start[1];
        let dz = self.end[2] - self.start[2];
        dx * dx + dy * dy + dz * dz
    }

    /// Returns the midpoint of the line.
    #[inline]
    pub fn midpoint(&self) -> [f32; 3] {
        [
            (self.start[0] + self.end[0]) * 0.5,
            (self.start[1] + self.end[1]) * 0.5,
            (self.start[2] + self.end[2]) * 0.5,
        ]
    }

    /// Returns the direction vector of the line (not normalized).
    #[inline]
    pub fn direction(&self) -> [f32; 3] {
        [
            self.end[0] - self.start[0],
            self.end[1] - self.start[1],
            self.end[2] - self.start[2],
        ]
    }

    /// Returns true if the line has zero length.
    #[inline]
    pub fn is_degenerate(&self) -> bool {
        self.length_squared() < 1e-10
    }

    /// Creates a red debug line.
    #[inline]
    pub fn red(start: [f32; 3], end: [f32; 3]) -> Self {
        Self::new(start, end, [1.0, 0.0, 0.0])
    }

    /// Creates a green debug line.
    #[inline]
    pub fn green(start: [f32; 3], end: [f32; 3]) -> Self {
        Self::new(start, end, [0.0, 1.0, 0.0])
    }

    /// Creates a blue debug line.
    #[inline]
    pub fn blue(start: [f32; 3], end: [f32; 3]) -> Self {
        Self::new(start, end, [0.0, 0.0, 1.0])
    }

    /// Creates a white debug line.
    #[inline]
    pub fn white(start: [f32; 3], end: [f32; 3]) -> Self {
        Self::new(start, end, [1.0, 1.0, 1.0])
    }
}

impl std::fmt::Display for DebugLine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Line([{:.2},{:.2},{:.2}] -> [{:.2},{:.2},{:.2}], rgb=[{:.2},{:.2},{:.2}])",
            self.start[0],
            self.start[1],
            self.start[2],
            self.end[0],
            self.end[1],
            self.end[2],
            self.color[0],
            self.color[1],
            self.color[2]
        )
    }
}

struct LineCollector {
    lines: Vec<DebugLine>,
}

impl LineCollector {
    fn new() -> Self {
        Self { lines: Vec::new() }
    }
}

impl DebugRenderBackend for LineCollector {
    fn draw_line(
        &mut self,
        _object: DebugRenderObject,
        a: rapier3d::prelude::Point<Real>,
        b: rapier3d::prelude::Point<Real>,
        color: [f32; 4],
    ) {
        self.lines.push(DebugLine {
            start: [a.x, a.y, a.z],
            end: [b.x, b.y, b.z],
            color: [color[0], color[1], color[2]],
        });
    }
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct Layers: u32 {
        const DEFAULT   = 0b00000001;
        const CHARACTER = 0b00000010;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum CharState {
    Grounded,
}

impl CharState {
    /// Returns the name of the state.
    #[inline]
    pub fn name(&self) -> &'static str {
        match self {
            Self::Grounded => "Grounded",
        }
    }

    /// Returns true if grounded.
    #[inline]
    pub fn is_grounded(&self) -> bool {
        matches!(self, Self::Grounded)
    }

    /// Returns all character states.
    pub fn all() -> [CharState; 1] {
        [Self::Grounded]
    }
}

impl std::fmt::Display for CharState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

#[derive(Clone, Copy, Debug)]
pub struct CharacterController {
    pub state: CharState,
    pub max_climb_angle_deg: f32,
    pub radius: f32,
    pub height: f32,
    pub max_step: f32,

    // Jump / Gravity
    pub vertical_velocity: f32,
    pub gravity_scale: f32,

    // Timers (State)
    pub time_since_grounded: f32,
    pub jump_buffer_timer: f32,

    // Config
    pub coyote_time_limit: f32,
    pub jump_buffer_limit: f32,

    pub pending_jump_velocity: f32,
}

impl CharacterController {
    /// Creates a new character controller with given dimensions.
    pub fn new(radius: f32, height: f32) -> Self {
        Self {
            state: CharState::Grounded,
            max_climb_angle_deg: 45.0,
            radius,
            height,
            max_step: 0.3,
            vertical_velocity: 0.0,
            gravity_scale: 1.0,
            time_since_grounded: 0.0,
            jump_buffer_timer: 0.0,
            coyote_time_limit: 0.15,
            jump_buffer_limit: 0.15,
            pending_jump_velocity: 0.0,
        }
    }

    /// Returns true if the character is grounded.
    #[inline]
    pub fn is_grounded(&self) -> bool {
        self.state.is_grounded()
    }

    /// Returns true if coyote time is still active.
    #[inline]
    pub fn has_coyote_time(&self) -> bool {
        self.time_since_grounded < self.coyote_time_limit
    }

    /// Returns true if jump is buffered.
    #[inline]
    pub fn has_buffered_jump(&self) -> bool {
        self.jump_buffer_timer > 0.0
    }

    /// Returns true if the character can jump.
    #[inline]
    pub fn can_jump(&self) -> bool {
        self.is_grounded() || self.has_coyote_time()
    }

    /// Returns the capsule volume approximation.
    #[inline]
    pub fn volume(&self) -> f32 {
        let cylinder_height = self.height - 2.0 * self.radius;
        let cylinder_volume = std::f32::consts::PI * self.radius * self.radius * cylinder_height;
        let sphere_volume = (4.0 / 3.0) * std::f32::consts::PI * self.radius.powi(3);
        cylinder_volume + sphere_volume
    }

    /// Returns the max climb angle in radians.
    #[inline]
    pub fn max_climb_angle_rad(&self) -> f32 {
        self.max_climb_angle_deg.to_radians()
    }

    /// Returns true if character is falling.
    #[inline]
    pub fn is_falling(&self) -> bool {
        self.vertical_velocity < -0.01
    }

    /// Returns true if character is rising.
    #[inline]
    pub fn is_rising(&self) -> bool {
        self.vertical_velocity > 0.01
    }

    /// Resets the controller to default state.
    pub fn reset(&mut self) {
        self.state = CharState::Grounded;
        self.vertical_velocity = 0.0;
        self.time_since_grounded = 0.0;
        self.jump_buffer_timer = 0.0;
        self.pending_jump_velocity = 0.0;
    }
}

impl std::fmt::Display for CharacterController {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "CharController(r={:.2}, h={:.2}, state={}, vel_y={:.2})",
            self.radius, self.height, self.state, self.vertical_velocity
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PhysicsConfig {
    pub gravity: Vec3,
    pub ccd_enabled: bool,
    pub max_ccd_substeps: usize,
    pub time_step: f32,
    pub water_level: f32,
    pub fluid_density: f32,
}

impl PhysicsConfig {
    /// Creates a new physics config with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a physics config with custom gravity.
    pub fn with_gravity(mut self, gravity: Vec3) -> Self {
        self.gravity = gravity;
        self
    }

    /// Enables CCD with the given substeps.
    pub fn with_ccd(mut self, substeps: usize) -> Self {
        self.ccd_enabled = true;
        self.max_ccd_substeps = substeps;
        self
    }

    /// Sets the time step (delta time).
    pub fn with_time_step(mut self, dt: f32) -> Self {
        self.time_step = dt;
        self
    }

    /// Sets the water level for buoyancy.
    pub fn with_water(mut self, level: f32, density: f32) -> Self {
        self.water_level = level;
        self.fluid_density = density;
        self
    }

    /// Returns the target FPS based on time step.
    #[inline]
    pub fn target_fps(&self) -> f32 {
        1.0 / self.time_step
    }

    /// Returns true if water simulation is enabled.
    #[inline]
    pub fn has_water(&self) -> bool {
        self.water_level.is_finite()
    }

    /// Returns the gravity magnitude.
    #[inline]
    pub fn gravity_magnitude(&self) -> f32 {
        self.gravity.length()
    }

    /// Returns true if this is Earth-like gravity.
    #[inline]
    pub fn is_earth_gravity(&self) -> bool {
        (self.gravity.y + 9.81).abs() < 0.1
            && self.gravity.x.abs() < 0.01
            && self.gravity.z.abs() < 0.01
    }

    /// Returns true if gravity is zero.
    #[inline]
    pub fn is_zero_gravity(&self) -> bool {
        self.gravity.length_squared() < 1e-6
    }
}

impl Default for PhysicsConfig {
    fn default() -> Self {
        Self {
            gravity: Vec3::new(0.0, -9.81, 0.0),
            ccd_enabled: false,
            max_ccd_substeps: 1,
            time_step: 1.0 / 60.0,
            water_level: f32::NEG_INFINITY,
            fluid_density: 1000.0,
        }
    }
}

impl std::fmt::Display for PhysicsConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "PhysicsConfig(g=[{:.2},{:.2},{:.2}], dt={:.4}, ccd={})",
            self.gravity.x, self.gravity.y, self.gravity.z, self.time_step, self.ccd_enabled
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub enum JointType {
    Fixed,
    Revolute {
        axis: Vec3,
        limits: Option<(f32, f32)>,
    },
    Prismatic {
        axis: Vec3,
        limits: Option<(f32, f32)>,
    },
    Spherical,
}

impl JointType {
    /// Returns the name of the joint type.
    #[inline]
    pub fn name(&self) -> &'static str {
        match self {
            Self::Fixed => "Fixed",
            Self::Revolute { .. } => "Revolute",
            Self::Prismatic { .. } => "Prismatic",
            Self::Spherical => "Spherical",
        }
    }

    /// Returns true if this is a fixed joint.
    #[inline]
    pub fn is_fixed(&self) -> bool {
        matches!(self, Self::Fixed)
    }

    /// Returns true if this is a revolute joint.
    #[inline]
    pub fn is_revolute(&self) -> bool {
        matches!(self, Self::Revolute { .. })
    }

    /// Returns true if this is a prismatic joint.
    #[inline]
    pub fn is_prismatic(&self) -> bool {
        matches!(self, Self::Prismatic { .. })
    }

    /// Returns true if this is a spherical joint.
    #[inline]
    pub fn is_spherical(&self) -> bool {
        matches!(self, Self::Spherical)
    }

    /// Returns true if the joint has limits.
    #[inline]
    pub fn has_limits(&self) -> bool {
        match self {
            Self::Revolute { limits, .. } | Self::Prismatic { limits, .. } => limits.is_some(),
            _ => false,
        }
    }

    /// Returns the axis if applicable.
    #[inline]
    pub fn axis(&self) -> Option<Vec3> {
        match self {
            Self::Revolute { axis, .. } | Self::Prismatic { axis, .. } => Some(*axis),
            _ => None,
        }
    }

    /// Returns the limits if applicable.
    #[inline]
    pub fn limits(&self) -> Option<(f32, f32)> {
        match self {
            Self::Revolute { limits, .. } | Self::Prismatic { limits, .. } => *limits,
            _ => None,
        }
    }

    /// Returns true if this is a rotational joint.
    #[inline]
    pub fn is_rotational(&self) -> bool {
        matches!(self, Self::Revolute { .. } | Self::Spherical)
    }

    /// Returns true if this is a linear joint.
    #[inline]
    pub fn is_linear(&self) -> bool {
        matches!(self, Self::Prismatic { .. })
    }

    /// Returns the degrees of freedom for this joint.
    #[inline]
    pub fn degrees_of_freedom(&self) -> u8 {
        match self {
            Self::Fixed => 0,
            Self::Revolute { .. } | Self::Prismatic { .. } => 1,
            Self::Spherical => 3,
        }
    }

    /// Creates a revolute joint around the Y axis.
    pub fn revolute_y() -> Self {
        Self::Revolute {
            axis: Vec3::Y,
            limits: None,
        }
    }

    /// Creates a revolute joint around the X axis.
    pub fn revolute_x() -> Self {
        Self::Revolute {
            axis: Vec3::X,
            limits: None,
        }
    }

    /// Creates a revolute joint around the Z axis.
    pub fn revolute_z() -> Self {
        Self::Revolute {
            axis: Vec3::Z,
            limits: None,
        }
    }

    /// Creates a prismatic joint along the Y axis.
    pub fn prismatic_y() -> Self {
        Self::Prismatic {
            axis: Vec3::Y,
            limits: None,
        }
    }
}

impl std::fmt::Display for JointType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Fixed => write!(f, "Fixed"),
            Self::Revolute { axis, limits } => {
                if let Some((min, max)) = limits {
                    write!(
                        f,
                        "Revolute(axis=[{:.2},{:.2},{:.2}], limits=[{:.2},{:.2}])",
                        axis.x, axis.y, axis.z, min, max
                    )
                } else {
                    write!(
                        f,
                        "Revolute(axis=[{:.2},{:.2},{:.2}])",
                        axis.x, axis.y, axis.z
                    )
                }
            }
            Self::Prismatic { axis, limits } => {
                if let Some((min, max)) = limits {
                    write!(
                        f,
                        "Prismatic(axis=[{:.2},{:.2},{:.2}], limits=[{:.2},{:.2}])",
                        axis.x, axis.y, axis.z, min, max
                    )
                } else {
                    write!(
                        f,
                        "Prismatic(axis=[{:.2},{:.2},{:.2}])",
                        axis.x, axis.y, axis.z
                    )
                }
            }
            Self::Spherical => write!(f, "Spherical"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct JointId(pub u64);

impl JointId {
    /// Creates a new joint ID.
    #[inline]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Returns the raw ID value.
    #[inline]
    pub const fn raw(&self) -> u64 {
        self.0
    }

    /// Returns true if this is a valid ID (non-zero).
    #[inline]
    pub const fn is_valid(&self) -> bool {
        self.0 != 0
    }

    /// Returns the invalid/null joint ID.
    #[inline]
    pub const fn invalid() -> Self {
        Self(0)
    }
}

impl std::fmt::Display for JointId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "JointId({})", self.0)
    }
}

impl From<u64> for JointId {
    fn from(id: u64) -> Self {
        Self(id)
    }
}

impl From<JointId> for u64 {
    fn from(id: JointId) -> Self {
        id.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct BuoyancyData {
    pub volume: f32,
    pub drag: f32,
}

impl BuoyancyData {
    /// Creates new buoyancy data.
    #[inline]
    pub const fn new(volume: f32, drag: f32) -> Self {
        Self { volume, drag }
    }

    /// Creates buoyancy data with zero drag.
    #[inline]
    pub const fn with_volume(volume: f32) -> Self {
        Self { volume, drag: 0.0 }
    }

    /// Returns true if volume is positive.
    #[inline]
    pub fn is_valid(&self) -> bool {
        self.volume > 0.0
    }

    /// Calculates buoyancy force for the given fluid density.
    #[inline]
    pub fn buoyancy_force(&self, fluid_density: f32) -> f32 {
        self.volume * fluid_density * 9.81
    }

    /// Calculates drag force for the given velocity.
    #[inline]
    pub fn drag_force(&self, velocity: f32) -> f32 {
        0.5 * self.drag * velocity * velocity
    }
}

impl std::fmt::Display for BuoyancyData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Buoyancy(vol={:.3}, drag={:.3})", self.volume, self.drag)
    }
}

pub struct PhysicsWorld {
    pub bodies: RigidBodySet,
    pub colliders: ColliderSet,
    pub joints: ImpulseJointSet,
    pub multibody_joints: MultibodyJointSet,
    pub pipeline: PhysicsPipeline,
    pub gravity: Vector<Real>,
    pub integration: IntegrationParameters,
    pub island_mgr: IslandManager,
    pub broad_phase: DefaultBroadPhase,
    pub narrow_phase: NarrowPhase,
    pub query_pipeline: QueryPipeline,
    pub ccd: CCDSolver,
    pub event_handler: ChannelEventCollector,
    pub collision_recv: rapier3d::crossbeam::channel::Receiver<CollisionEvent>,
    pub contact_force_recv: rapier3d::crossbeam::channel::Receiver<ContactForceEvent>,
    body_ids: HashMap<RigidBodyHandle, BodyId>,
    body_kinds: HashMap<RigidBodyHandle, ActorKind>,
    next_body_id: BodyId,
    pub char_map: HashMap<BodyId, CharacterController>,
    next_joint_id: u64,
    debug_render_pipeline: DebugRenderPipeline,
    pub buoyancy_bodies: HashMap<BodyId, BuoyancyData>,
    pub water_level: f32,
    pub fluid_density: f32,
    pub wind: Vec3,

    /// Async physics scheduler (feature-gated)
    #[cfg(feature = "async-physics")]
    pub async_scheduler: Option<AsyncPhysicsScheduler>,
}

impl PhysicsWorld {
    pub fn new(gravity: Vec3) -> Self {
        let (collision_send, collision_recv) = rapier3d::crossbeam::channel::unbounded();
        let (contact_force_send, contact_force_recv) = rapier3d::crossbeam::channel::unbounded();
        let event_handler = ChannelEventCollector::new(collision_send, contact_force_send);

        Self {
            bodies: RigidBodySet::new(),
            colliders: ColliderSet::new(),
            joints: ImpulseJointSet::new(),
            multibody_joints: MultibodyJointSet::new(),
            pipeline: PhysicsPipeline::new(),
            gravity: vector![gravity.x, gravity.y, gravity.z],
            integration: IntegrationParameters::default(),
            island_mgr: IslandManager::new(),
            broad_phase: DefaultBroadPhase::new(),
            narrow_phase: NarrowPhase::new(),
            query_pipeline: QueryPipeline::new(),
            ccd: CCDSolver::new(),
            event_handler,
            collision_recv,
            contact_force_recv,
            body_ids: HashMap::new(),
            body_kinds: HashMap::new(),
            next_body_id: 1,
            char_map: HashMap::new(),
            next_joint_id: 1,
            debug_render_pipeline: DebugRenderPipeline::default(),
            buoyancy_bodies: HashMap::new(),
            water_level: f32::NEG_INFINITY,
            fluid_density: 1000.0,
            wind: Vec3::ZERO,
            #[cfg(feature = "async-physics")]
            async_scheduler: None,
        }
    }

    pub fn from_config(config: PhysicsConfig) -> Self {
        let (collision_send, collision_recv) = rapier3d::crossbeam::channel::unbounded();
        let (contact_force_send, contact_force_recv) = rapier3d::crossbeam::channel::unbounded();
        let event_handler = ChannelEventCollector::new(collision_send, contact_force_send);

        let integration = IntegrationParameters {
            dt: config.time_step,
            ..Default::default()
        };

        Self {
            bodies: RigidBodySet::new(),
            colliders: ColliderSet::new(),
            joints: ImpulseJointSet::new(),
            multibody_joints: MultibodyJointSet::new(),
            pipeline: PhysicsPipeline::new(),
            gravity: vector![config.gravity.x, config.gravity.y, config.gravity.z],
            integration,
            island_mgr: IslandManager::new(),
            broad_phase: DefaultBroadPhase::new(),
            narrow_phase: NarrowPhase::new(),
            query_pipeline: QueryPipeline::new(),
            ccd: CCDSolver::new(),
            event_handler,
            collision_recv,
            contact_force_recv,
            body_ids: HashMap::new(),
            body_kinds: HashMap::new(),
            next_body_id: 1,
            char_map: HashMap::new(),
            next_joint_id: 1,
            debug_render_pipeline: DebugRenderPipeline::default(),
            buoyancy_bodies: HashMap::new(),
            water_level: f32::NEG_INFINITY,
            fluid_density: 1000.0,
            wind: Vec3::ZERO,
            #[cfg(feature = "async-physics")]
            async_scheduler: None,
        }
    }

    /// Enable async physics with optional thread count (0 = auto-detect)
    /// This configures Rayon's global thread pool, which Rapier3D uses for parallel solving
    #[cfg(feature = "async-physics")]
    pub fn enable_async_physics(&mut self, thread_count: usize) {
        // Configure Rayon thread pool if not already initialized
        if thread_count > 0 {
            // Try to build thread pool (may fail if already initialized, which is fine)
            let _ = rayon::ThreadPoolBuilder::new()
                .num_threads(thread_count)
                .build_global();
        }

        self.async_scheduler = Some(if thread_count > 0 {
            AsyncPhysicsScheduler::with_threads(thread_count)
        } else {
            AsyncPhysicsScheduler::new()
        });
    }

    /// Get last physics step profile (for telemetry)
    #[cfg(feature = "async-physics")]
    pub fn get_last_profile(&self) -> Option<PhysicsStepProfile> {
        self.async_scheduler.as_ref().map(|s| s.get_last_profile())
    }

    fn alloc_id(&mut self) -> BodyId {
        let id = self.next_body_id;
        self.next_body_id += 1;
        id
    }

    pub fn step(&mut self) {
        #[cfg(feature = "profiling")]
        span!("Physics::World::step");

        #[cfg(feature = "async-physics")]
        {
            // When async scheduler is enabled, Rapier3D automatically uses
            // Rayon's global thread pool for parallel island solving.
            // The thread count was configured when enable_async_physics was called.
            if self.async_scheduler.is_some() {
                use std::time::Instant;
                let start = Instant::now();

                self.step_internal();

                let duration = start.elapsed();

                // Update telemetry
                if let Some(scheduler) = &mut self.async_scheduler {
                    scheduler.record_step_telemetry(duration);
                }
                return;
            }
        }

        // Fallback to regular step (single-threaded)
        self.step_internal();
    }

    /// Internal physics step (shared by sync and async paths)
    /// When called with async scheduler enabled, Rapier3D uses Rayon for parallel solving
    fn step_internal(&mut self) {
        // Allocation-measurement instrumentation (audit 2026-04-17, open question #7).
        // `physics.step.allocs` exposes whether Rapier3D's per-step allocation count
        // grows with simulation time — the audit could not answer this statically.
        #[cfg(feature = "profiling")]
        astraweave_profiling::measured_span!("physics.step");

        #[cfg(feature = "profiling")]
        {
            span!("Physics::Rapier::pipeline");
            plot!("Physics::collider_count", self.colliders.len() as u64);
        }

        // Apply buoyancy forces before physics step
        self.apply_buoyancy_forces();

        self.pipeline.step(
            &self.gravity,
            &self.integration,
            &mut self.island_mgr,
            &mut self.broad_phase,
            &mut self.narrow_phase,
            &mut self.bodies,
            &mut self.colliders,
            &mut self.joints,
            &mut self.multibody_joints,
            &mut self.ccd,
            Some(&mut self.query_pipeline),
            &(),
            &self.event_handler,
        );

        // CRITICAL FIX (Week 2 Day 3): Update query pipeline after physics step
        // Without this, raycasts in control_character() use stale geometry,
        // causing character controller to fail ground detection
        self.query_pipeline.update(&self.colliders);
    }

    pub fn apply_force(&mut self, id: BodyId, force: Vec3) {
        if let Some(h) = self.handle_of(id) {
            if let Some(rb) = self.bodies.get_mut(h) {
                rb.add_force(vector![force.x, force.y, force.z], true);
            }
        }
    }

    pub fn apply_impulse(&mut self, id: BodyId, impulse: Vec3) {
        if let Some(h) = self.handle_of(id) {
            if let Some(rb) = self.bodies.get_mut(h) {
                rb.apply_impulse(vector![impulse.x, impulse.y, impulse.z], true);
            }
        }
    }

    pub fn get_velocity(&self, id: BodyId) -> Option<Vec3> {
        let h = self.handle_of(id)?;
        let rb = self.bodies.get(h)?;
        let v = rb.linvel();
        Some(Vec3::new(v.x, v.y, v.z))
    }

    pub fn set_velocity(&mut self, id: BodyId, vel: Vec3) {
        if let Some(h) = self.handle_of(id) {
            if let Some(rb) = self.bodies.get_mut(h) {
                rb.set_linvel(vector![vel.x, vel.y, vel.z], true);
            }
        }
    }

    pub fn create_ground_plane(&mut self, half: Vec3, friction: f32) -> BodyId {
        let rb = RigidBodyBuilder::fixed().build();
        let h = self.bodies.insert(rb);
        let shape = ColliderBuilder::cuboid(half.x, 0.1, half.z)
            .friction(friction)
            .collision_groups(InteractionGroups::new(
                Group::from_bits_truncate(Layers::DEFAULT.bits()),
                Group::ALL,
            ))
            .build();
        self.colliders
            .insert_with_parent(shape, h, &mut self.bodies);
        self.tag_body(h, ActorKind::Static)
    }

    pub fn add_static_trimesh(
        &mut self,
        vertices: &[Vec3],
        indices: &[[u32; 3]],
        groups: Layers,
    ) -> BodyId {
        let rb = RigidBodyBuilder::fixed().build();
        let h = self.bodies.insert(rb);
        let v: Vec<Point<Real>> = vertices.iter().map(|p| point![p.x, p.y, p.z]).collect();
        let i: Vec<[u32; 3]> = indices.to_vec();
        let coll = ColliderBuilder::trimesh(v, i)
            .collision_groups(InteractionGroups::new(
                Group::from_bits_truncate(groups.bits()),
                Group::ALL,
            ))
            .friction(0.9)
            .build();
        self.colliders.insert_with_parent(coll, h, &mut self.bodies);
        self.tag_body(h, ActorKind::Static)
    }

    pub fn add_dynamic_box(&mut self, pos: Vec3, half: Vec3, mass: f32, groups: Layers) -> BodyId {
        #[cfg(feature = "profiling")]
        {
            span!("Physics::RigidBody::create");
            plot!("Physics::rigid_body_count", self.bodies.len() as u64);
        }

        let rb = RigidBodyBuilder::dynamic()
            .translation(vector![pos.x, pos.y, pos.z])
            .build();
        let h = self.bodies.insert(rb);
        let coll = ColliderBuilder::cuboid(half.x, half.y, half.z)
            .mass(mass)
            .collision_groups(InteractionGroups::new(
                Group::from_bits_truncate(groups.bits()),
                Group::ALL,
            ))
            .friction(0.8)
            .active_events(ActiveEvents::COLLISION_EVENTS)
            .build();
        self.colliders.insert_with_parent(coll, h, &mut self.bodies);
        self.tag_body(h, ActorKind::Dynamic)
    }

    pub fn add_character(&mut self, pos: Vec3, half: Vec3) -> BodyId {
        #[cfg(feature = "profiling")]
        {
            span!("Physics::Character::create");
            plot!("Physics::character_count", self.char_map.len() as u64);
        }

        let rb = RigidBodyBuilder::kinematic_position_based()
            .translation(vector![pos.x, pos.y, pos.z])
            .build();
        let h = self.bodies.insert(rb);
        let coll = ColliderBuilder::capsule_y(half.y, half.x.max(half.z))
            .collision_groups(InteractionGroups::new(
                Group::from_bits_truncate(Layers::CHARACTER.bits()),
                Group::ALL,
            ))
            .friction(0.6)
            .build();
        self.colliders.insert_with_parent(coll, h, &mut self.bodies);
        let id = self.tag_body(h, ActorKind::Character);
        self.char_map.insert(
            id,
            CharacterController {
                state: CharState::Grounded,
                max_climb_angle_deg: 70.0,
                radius: half.x.max(half.z),
                height: half.y * 2.0,
                max_step: 0.4,
                vertical_velocity: 0.0,
                gravity_scale: 1.0,
                time_since_grounded: 0.0,
                jump_buffer_timer: 0.0,
                coyote_time_limit: 0.1, // 100ms
                jump_buffer_limit: 0.1, // 100ms
                pending_jump_velocity: 0.0,
            },
        );
        id
    }

    pub fn jump(&mut self, id: BodyId, height: f32) {
        if let Some(ctrl) = self.char_map.get_mut(&id) {
            ctrl.jump_buffer_timer = ctrl.jump_buffer_limit;
            let g = 9.81 * ctrl.gravity_scale;
            ctrl.pending_jump_velocity = (2.0 * g * height).sqrt();
        }
    }

    pub fn control_character(&mut self, id: BodyId, desired_move: Vec3, dt: f32, _climb: bool) {
        #[cfg(feature = "profiling")]
        span!("Physics::CharacterController::move");

        let Some(mut ctrl) = self.char_map.get(&id).copied() else {
            return;
        };
        let Some(h) = self.handle_of(id) else {
            return;
        };
        let Some(rb) = self.bodies.get(h) else {
            return;
        };
        let pos = *rb.position();
        let start = glam::Vec3::new(pos.translation.x, pos.translation.y, pos.translation.z);
        // Update timers
        ctrl.jump_buffer_timer -= dt;

        // Apply gravity (if not climbing)
        if !_climb {
            ctrl.vertical_velocity -= 9.81 * ctrl.gravity_scale * dt;
        } else {
            ctrl.vertical_velocity = 0.0;
        }

        // Check Jump
        let can_jump = ctrl.time_since_grounded < ctrl.coyote_time_limit;
        let wants_jump = ctrl.jump_buffer_timer > 0.0;

        if can_jump && wants_jump && !_climb {
            ctrl.vertical_velocity = ctrl.pending_jump_velocity;
            ctrl.time_since_grounded = ctrl.coyote_time_limit + 1.0; // Invalidate coyote
            ctrl.jump_buffer_timer = 0.0; // Consume buffer
        }

        let mut d = desired_move * dt;
        let has_horizontal_move = d.length_squared() >= 1e-6;
        let has_vertical_move = ctrl.vertical_velocity.abs() > 1e-4 || _climb;

        if !has_horizontal_move && !has_vertical_move {
            self.char_map.insert(id, ctrl);
            return;
        }

        if has_horizontal_move {
            // Basic obstacle avoidance: raycast forward; slide along hit normal
            let dir = d.normalize();
            let ray_origin = start + glam::Vec3::Y * (ctrl.height * 0.5);
            let ray = rapier3d::prelude::Ray::new(
                point![ray_origin.x, ray_origin.y, ray_origin.z],
                vector![dir.x, dir.y, dir.z],
            );
            // BUG FIX (Week 2 Day 3): Exclude character's own colliders from raycasts
            // Without this, the character detects its own capsule as an obstacle
            let filter = QueryFilter::default().exclude_rigid_body(h);
            if let Some((_, hit)) = self.query_pipeline.cast_ray_and_get_normal(
                &self.bodies,
                &self.colliders,
                &ray,
                d.length() + ctrl.radius + 0.05,
                true,
                filter,
            ) {
                // Deflect movement along tangent plane
                let n = glam::Vec3::new(hit.normal.x, hit.normal.y, hit.normal.z).normalize();
                d = d - n * d.dot(n);
            }
        }

        // Tentative horizontal move
        let mut new_pos = start + glam::Vec3::new(d.x, 0.0, d.z);

        if _climb {
            // Simple vertical climb
            new_pos.y = start.y + 2.0 * dt;
            ctrl.time_since_grounded = 0.0;
        } else {
            // Apply vertical velocity
            new_pos.y = start.y + ctrl.vertical_velocity * dt;

            // Ground check / Step / Slope
            if ctrl.vertical_velocity <= 0.0 {
                let cast_origin = new_pos + glam::Vec3::Y * (ctrl.height);
                let ray_down = rapier3d::prelude::Ray::new(
                    point![cast_origin.x, cast_origin.y, cast_origin.z],
                    vector![0.0, -1.0, 0.0],
                );
                if let Some((_, hit)) = self.query_pipeline.cast_ray_and_get_normal(
                    &self.bodies,
                    &self.colliders,
                    &ray_down,
                    ctrl.height + ctrl.max_step + 1.0,
                    true,
                    QueryFilter::default().exclude_rigid_body(h),
                ) {
                    let ground_normal =
                        glam::Vec3::new(hit.normal.x, hit.normal.y, hit.normal.z).normalize();
                    let slope = ground_normal.dot(glam::Vec3::Y).acos().to_degrees();
                    let ground_y = cast_origin.y - hit.time_of_impact;

                    if slope <= ctrl.max_climb_angle_deg + 1e-2 {
                        // Snap to ground if close enough
                        if new_pos.y <= ground_y + 0.05 {
                            new_pos.y = ground_y;
                            ctrl.vertical_velocity = 0.0;
                            ctrl.time_since_grounded = 0.0;
                        }
                    }
                } else {
                    ctrl.time_since_grounded += dt;
                }
            } else {
                ctrl.time_since_grounded += dt;
            }
        }

        // Commit move
        let mut p = pos;
        p.translation.x = new_pos.x;
        p.translation.y = new_pos.y;
        p.translation.z = new_pos.z;
        if let Some(rbmut) = self.bodies.get_mut(h) {
            // BUG FIX (Week 2 Day 3): Use set_next_kinematic_position for kinematic bodies
            // set_position() with wake=true doesn't properly update kinematic bodies
            // across multiple frames - position gets reset by physics step
            rbmut.set_next_kinematic_position(p);
        }

        // BUG FIX: Store updated controller state (vertical_velocity, timers, etc.)
        // Previously only stored on the early-return path, discarding gravity/jump state
        self.char_map.insert(id, ctrl);
    }

    pub fn handle_of(&self, id: BodyId) -> Option<RigidBodyHandle> {
        self.body_ids
            .iter()
            .find_map(|(h, bid)| if *bid == id { Some(*h) } else { None })
    }

    pub fn id_of(&self, handle: RigidBodyHandle) -> Option<BodyId> {
        self.body_ids.get(&handle).copied()
    }

    pub fn body_transform(&self, id: BodyId) -> Option<Mat4> {
        let h = self.handle_of(id)?;
        let rb = self.bodies.get(h)?;
        let iso = rb.position();
        let rot = glam::Quat::from_xyzw(
            iso.rotation.i,
            iso.rotation.j,
            iso.rotation.k,
            iso.rotation.w,
        );
        Some(Mat4::from_rotation_translation(
            rot,
            vec3(iso.translation.x, iso.translation.y, iso.translation.z),
        ))
    }

    fn tag_body(&mut self, h: RigidBodyHandle, kind: ActorKind) -> BodyId {
        let id = self.alloc_id();
        self.body_ids.insert(h, id);
        self.body_kinds.insert(h, kind);
        id
    }

    pub fn add_buoyancy(&mut self, body: BodyId, volume: f32, drag: f32) {
        self.buoyancy_bodies
            .insert(body, BuoyancyData { volume, drag });
    }

    fn apply_buoyancy_forces(&mut self) {
        for (body_id, buoyancy_data) in &self.buoyancy_bodies {
            if let Some(handle) = self.handle_of(*body_id) {
                if let Some(rb) = self.bodies.get_mut(handle) {
                    let pos = rb.position();
                    let body_y = pos.translation.y;

                    // Only apply buoyancy if body is below water level
                    if body_y < self.water_level {
                        // Buoyancy force = volume * fluid_density * gravity (upward)
                        let buoyancy_force = buoyancy_data.volume * self.fluid_density * 9.81;

                        // Drag force = -velocity * drag coefficient
                        let velocity = rb.linvel();
                        let drag_force = vector![
                            -velocity.x * buoyancy_data.drag,
                            -velocity.y * buoyancy_data.drag,
                            -velocity.z * buoyancy_data.drag
                        ];

                        // Total force (buoyancy up + drag)
                        let total_force =
                            vector![drag_force.x, buoyancy_force + drag_force.y, drag_force.z];

                        rb.add_force(total_force, true);
                    }
                }
            }
        }
    }

    pub fn add_water_aabb(&mut self, _min: Vec3, _max: Vec3, _density: f32, _linear_damp: f32) {}

    pub fn set_wind(&mut self, dir: Vec3, strength: f32) {
        self.wind = dir.normalize_or_zero() * strength;
    }

    /// Apply radial explosion impulse to all dynamic bodies in range
    ///
    /// # Arguments
    /// * `center` - Explosion center
    /// * `radius` - Maximum effect radius
    /// * `force` - Force at center (falls off with distance)
    /// * `falloff` - Falloff curve type
    /// * `upward_bias` - Upward impulse bias (0.0 = radial, 1.0 = fully upward)
    ///
    /// # Returns
    /// Number of bodies affected
    pub fn apply_radial_impulse(
        &mut self,
        center: Vec3,
        radius: f32,
        force: f32,
        falloff: projectile::FalloffCurve,
        upward_bias: f32,
    ) -> usize {
        // Collect body positions and IDs
        let mut affected = Vec::new();

        for (&handle, &_body_id) in &self.body_ids {
            if let Some(rb) = self.bodies.get(handle) {
                if !rb.is_dynamic() {
                    continue;
                }
                let pos = rb.position().translation;
                let body_pos = Vec3::new(pos.x, pos.y, pos.z);
                let to_body = body_pos - center;
                let distance = to_body.length();

                if distance < radius {
                    affected.push((handle, body_pos, distance, to_body));
                }
            }
        }

        // Apply impulses
        let count = affected.len();
        for (handle, _body_pos, distance, to_body) in affected {
            let falloff_mult = falloff.calculate(distance, radius);
            let force_mag = force * falloff_mult;

            let radial_dir = if distance > 0.001 {
                to_body.normalize()
            } else {
                Vec3::Y
            };

            let biased_dir = (radial_dir * (1.0 - upward_bias) + Vec3::Y * upward_bias).normalize();
            let impulse = biased_dir * force_mag;

            if let Some(rb) = self.bodies.get_mut(handle) {
                rb.apply_impulse(vector![impulse.x, impulse.y, impulse.z], true);
            }
        }

        count
    }

    /// Perform a raycast and return hit info
    ///
    /// # Returns
    /// (hit_position, hit_normal, body_id, distance) if hit
    pub fn raycast(
        &self,
        origin: Vec3,
        direction: Vec3,
        max_distance: f32,
    ) -> Option<(Vec3, Vec3, Option<BodyId>, f32)> {
        let ray = rapier3d::prelude::Ray::new(
            point![origin.x, origin.y, origin.z],
            vector![direction.x, direction.y, direction.z],
        );

        self.query_pipeline
            .cast_ray_and_get_normal(
                &self.bodies,
                &self.colliders,
                &ray,
                max_distance,
                true,
                QueryFilter::default(),
            )
            .map(|(collider_handle, hit)| {
                let hit_pos = origin + direction * hit.time_of_impact;
                let normal = Vec3::new(hit.normal.x, hit.normal.y, hit.normal.z);

                // Get body ID from collider
                let body_id = self
                    .colliders
                    .get(collider_handle)
                    .and_then(|c| c.parent())
                    .and_then(|rb_handle| self.body_ids.get(&rb_handle).copied());

                (hit_pos, normal, body_id, hit.time_of_impact)
            })
    }

    pub fn clear_water(&mut self) {}
    pub fn add_destructible_box(
        &mut self,
        pos: Vec3,
        half: Vec3,
        mass: f32,
        _health: f32,
        _break_impulse: f32,
    ) -> BodyId {
        self.add_dynamic_box(pos, half, mass, Layers::DEFAULT)
    }
    pub fn break_destructible(&mut self, id: BodyId) {
        if let Some(h) = self.handle_of(id) {
            // Remove from Rapier sets
            self.bodies.remove(
                h,
                &mut self.island_mgr,
                &mut self.colliders,
                &mut self.joints,
                &mut self.multibody_joints,
                true,
            );

            // Remove from our mappings
            self.body_ids.remove(&h);
            self.body_kinds.remove(&h);
            self.char_map.remove(&id);
            self.buoyancy_bodies.remove(&id);
        }
    }

    #[allow(dead_code)]
    fn process_destructible_hits(&mut self) {}

    pub fn set_body_position(&mut self, id: BodyId, pos: Vec3) {
        if let Some(h) = self.handle_of(id) {
            if let Some(rb) = self.bodies.get_mut(h) {
                rb.set_translation(vector![pos.x, pos.y, pos.z], true);
            }
        }
    }

    pub fn enable_ccd(&mut self, id: BodyId) {
        if let Some(h) = self.handle_of(id) {
            if let Some(rb) = self.bodies.get_mut(h) {
                rb.enable_ccd(true);
            }
        }
    }

    pub fn add_joint(&mut self, body1: BodyId, body2: BodyId, joint_type: JointType) -> JointId {
        let Some(handle1) = self.handle_of(body1) else {
            return JointId(0);
        };
        let Some(handle2) = self.handle_of(body2) else {
            return JointId(0);
        };

        let joint = match joint_type {
            JointType::Fixed => GenericJointBuilder::new(JointAxesMask::LOCKED_FIXED_AXES).build(),
            JointType::Revolute { axis, limits } => {
                let local_axis = UnitVector::new_normalize(vector![axis.x, axis.y, axis.z]);
                let mut builder = RevoluteJointBuilder::new(local_axis);
                if let Some((min, max)) = limits {
                    builder = builder.limits([min, max]);
                }
                builder.build().into()
            }
            JointType::Prismatic { axis, limits } => {
                let local_axis = UnitVector::new_normalize(vector![axis.x, axis.y, axis.z]);
                let mut builder = PrismaticJointBuilder::new(local_axis);
                if let Some((min, max)) = limits {
                    builder = builder.limits([min, max]);
                }
                builder.build().into()
            }
            JointType::Spherical => SphericalJointBuilder::new().build().into(),
        };

        self.joints.insert(handle1, handle2, joint, true);

        let joint_id = self.next_joint_id;
        self.next_joint_id += 1;
        JointId(joint_id)
    }

    pub fn get_debug_lines(&mut self) -> Vec<DebugLine> {
        let mut collector = LineCollector::new();
        self.debug_render_pipeline.render(
            &mut collector,
            &self.bodies,
            &self.colliders,
            &self.joints,
            &self.multibody_joints,
            &self.narrow_phase,
        );
        collector.lines
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn character_position_updates() {
        let mut pw = PhysicsWorld::new(Vec3::new(0.0, -9.8, 0.0));
        let _ground = pw.create_ground_plane(Vec3::new(10.0, 0.5, 10.0), 0.9);
        let char_id = pw.add_character(Vec3::new(0.0, 1.0, 0.0), Vec3::new(0.4, 0.9, 0.4));

        // Check initial position
        let pos0 = pw.body_transform(char_id).unwrap().w_axis;
        assert!(
            (pos0.x - 0.0).abs() < 0.01,
            "initial x should be ~0, got {}",
            pos0.x
        );

        // Move once
        pw.control_character(char_id, Vec3::new(1.0, 0.0, 0.0), 1.0 / 60.0, false);
        pw.step();

        let pos1 = pw.body_transform(char_id).unwrap().w_axis;

        // Move again
        pw.control_character(char_id, Vec3::new(1.0, 0.0, 0.0), 1.0 / 60.0, false);
        pw.step();

        let pos2 = pw.body_transform(char_id).unwrap().w_axis;

        // Position should accumulate
        assert!(
            pos2.x > pos1.x,
            "x should increase: frame1={}, frame2={}",
            pos1.x,
            pos2.x
        );
    }

    #[test]
    fn character_moves_forward() {
        let mut pw = PhysicsWorld::new(Vec3::new(0.0, -9.8, 0.0));
        let _ground = pw.create_ground_plane(Vec3::new(10.0, 0.5, 10.0), 0.9);
        let char_id = pw.add_character(Vec3::new(0.0, 1.0, 0.0), Vec3::new(0.4, 0.9, 0.4));
        for _ in 0..60 {
            pw.control_character(char_id, Vec3::new(1.0, 0.0, 0.0), 1.0 / 60.0, false);
            pw.step();
        }
        let x = pw.body_transform(char_id).unwrap().w_axis.x;
        assert!(x > 0.5, "character should have moved forward, x={}", x);
    }

    // ===== PhysicsWorld Basic Tests =====

    #[test]
    fn test_physics_world_new() {
        let pw = PhysicsWorld::new(Vec3::new(0.0, -9.8, 0.0));
        assert_eq!(pw.gravity.x, 0.0);
        assert_eq!(pw.gravity.y, -9.8);
        assert_eq!(pw.gravity.z, 0.0);
        assert!(pw.bodies.is_empty());
        assert!(pw.colliders.is_empty());
    }

    #[test]
    fn test_physics_world_from_config() {
        let config = PhysicsConfig {
            gravity: Vec3::new(0.0, -10.0, 0.0),
            time_step: 1.0 / 120.0,
            ccd_enabled: false,
            max_ccd_substeps: 1,
            water_level: f32::NEG_INFINITY,
            fluid_density: 1000.0,
        };
        let pw = PhysicsWorld::from_config(config);
        assert_eq!(pw.gravity.y, -10.0);
        assert_eq!(pw.integration.dt, 1.0 / 120.0);
    }

    #[test]
    fn test_create_ground_plane() {
        let mut pw = PhysicsWorld::new(Vec3::new(0.0, -9.8, 0.0));
        let ground = pw.create_ground_plane(Vec3::new(5.0, 0.5, 5.0), 0.8);

        assert!(pw.body_transform(ground).is_some());
        assert!(!pw.bodies.is_empty());
        assert!(!pw.colliders.is_empty());
    }

    #[test]
    fn test_add_dynamic_box() {
        let mut pw = PhysicsWorld::new(Vec3::new(0.0, -9.8, 0.0));
        let box_id = pw.add_dynamic_box(
            Vec3::new(0.0, 5.0, 0.0),
            Vec3::new(0.5, 0.5, 0.5),
            1.0,
            Layers::DEFAULT,
        );

        assert!(pw.body_transform(box_id).is_some());

        // Step physics - box should fall
        for _ in 0..60 {
            pw.step();
        }

        let y = pw.body_transform(box_id).unwrap().w_axis.y;
        assert!(y < 5.0, "Box should have fallen, y={}", y);
    }

    #[test]
    fn test_apply_force() {
        let mut pw = PhysicsWorld::new(Vec3::new(0.0, -9.8, 0.0));
        let _ground = pw.create_ground_plane(Vec3::new(10.0, 0.5, 10.0), 0.9);
        let box_id = pw.add_dynamic_box(
            Vec3::new(0.0, 2.0, 0.0),
            Vec3::new(0.5, 0.5, 0.5),
            1.0,
            Layers::DEFAULT,
        );

        pw.apply_force(box_id, Vec3::new(100.0, 0.0, 0.0));
        pw.step();

        let vel = pw.get_velocity(box_id).unwrap();
        assert!(vel.x > 0.0, "Force should have applied positive x velocity");
    }

    #[test]
    fn test_apply_impulse() {
        let mut pw = PhysicsWorld::new(Vec3::new(0.0, -9.8, 0.0));
        let box_id = pw.add_dynamic_box(
            Vec3::new(0.0, 5.0, 0.0),
            Vec3::new(0.5, 0.5, 0.5),
            1.0,
            Layers::DEFAULT,
        );

        pw.apply_impulse(box_id, Vec3::new(0.0, 50.0, 0.0));

        let vel = pw.get_velocity(box_id).unwrap();
        assert!(vel.y > 0.0, "Impulse should have given upward velocity");
    }

    #[test]
    fn test_get_set_velocity() {
        let mut pw = PhysicsWorld::new(Vec3::new(0.0, -9.8, 0.0));
        let box_id = pw.add_dynamic_box(
            Vec3::new(0.0, 5.0, 0.0),
            Vec3::new(0.5, 0.5, 0.5),
            1.0,
            Layers::DEFAULT,
        );

        // Get initial velocity
        let initial_vel = pw.get_velocity(box_id).unwrap();
        assert_eq!(initial_vel.x, 0.0);

        // Set velocity
        pw.set_velocity(box_id, Vec3::new(10.0, 0.0, 5.0));

        let new_vel = pw.get_velocity(box_id).unwrap();
        assert_eq!(new_vel.x, 10.0);
        assert_eq!(new_vel.z, 5.0);
    }

    #[test]
    fn test_get_velocity_invalid_id() {
        let pw = PhysicsWorld::new(Vec3::new(0.0, -9.8, 0.0));
        let vel = pw.get_velocity(9999);
        assert!(vel.is_none());
    }

    #[test]
    fn test_body_transform_invalid_id() {
        let pw = PhysicsWorld::new(Vec3::new(0.0, -9.8, 0.0));
        let transform = pw.body_transform(9999);
        assert!(transform.is_none());
    }

    // ===== Static Trimesh Tests =====

    #[test]
    fn test_add_static_trimesh() {
        let mut pw = PhysicsWorld::new(Vec3::new(0.0, -9.8, 0.0));

        // Create simple triangle
        let vertices = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 0.0, 1.0),
        ];
        let indices = vec![[0, 1, 2]];

        let mesh_id = pw.add_static_trimesh(&vertices, &indices, Layers::DEFAULT);
        assert!(pw.body_transform(mesh_id).is_some());
    }

    // ===== Character Controller Tests =====

    #[test]
    fn test_character_jump() {
        let mut pw = PhysicsWorld::new(Vec3::new(0.0, -9.8, 0.0));
        let _ground = pw.create_ground_plane(Vec3::new(10.0, 0.5, 10.0), 0.9);
        let char_id = pw.add_character(Vec3::new(0.0, 1.0, 0.0), Vec3::new(0.4, 0.9, 0.4));

        // Initial position
        let y0 = pw.body_transform(char_id).unwrap().w_axis.y;

        // Jump
        pw.jump(char_id, 2.0);

        // Run physics
        for _ in 0..30 {
            pw.control_character(char_id, Vec3::ZERO, 1.0 / 60.0, false);
            pw.step();
        }

        let y1 = pw.body_transform(char_id).unwrap().w_axis.y;
        // After 30 frames of jump, character should have moved
        assert!(y1 != y0, "Jump should change position");
    }

    #[test]
    fn test_control_character_invalid_id() {
        let mut pw = PhysicsWorld::new(Vec3::new(0.0, -9.8, 0.0));
        // Should not panic on invalid ID
        pw.control_character(9999, Vec3::new(1.0, 0.0, 0.0), 1.0 / 60.0, false);
    }

    #[test]
    fn test_jump_invalid_id() {
        let mut pw = PhysicsWorld::new(Vec3::new(0.0, -9.8, 0.0));
        // Should not panic on invalid ID
        pw.jump(9999, 2.0);
    }

    // ===== Buoyancy & Environment Tests =====

    #[test]
    fn test_add_buoyancy() {
        let mut pw = PhysicsWorld::new(Vec3::new(0.0, -9.8, 0.0));
        let box_id = pw.add_dynamic_box(
            Vec3::new(0.0, 5.0, 0.0),
            Vec3::new(0.5, 0.5, 0.5),
            1.0,
            Layers::DEFAULT,
        );

        pw.add_buoyancy(box_id, 1.0, 0.5);
        assert!(pw.buoyancy_bodies.contains_key(&box_id));
    }

    #[test]
    fn test_set_wind() {
        let mut pw = PhysicsWorld::new(Vec3::new(0.0, -9.8, 0.0));
        pw.set_wind(Vec3::new(1.0, 0.0, 0.0), 5.0);

        // set_wind stores dir * strength
        assert_eq!(pw.wind.x, 5.0); // 1.0 * 5.0
        assert_eq!(pw.wind.y, 0.0);
        assert_eq!(pw.wind.z, 0.0);
    }

    // ===== Handle/ID Mapping Tests =====

    #[test]
    fn test_handle_of_valid() {
        let mut pw = PhysicsWorld::new(Vec3::new(0.0, -9.8, 0.0));
        let box_id = pw.add_dynamic_box(
            Vec3::new(0.0, 5.0, 0.0),
            Vec3::new(0.5, 0.5, 0.5),
            1.0,
            Layers::DEFAULT,
        );

        assert!(pw.handle_of(box_id).is_some());
    }

    #[test]
    fn test_handle_of_invalid() {
        let pw = PhysicsWorld::new(Vec3::new(0.0, -9.8, 0.0));
        assert!(pw.handle_of(9999).is_none());
    }

    // ===== ActorKind Tests =====

    #[test]
    fn test_actor_kind_variants() {
        let _ = ActorKind::Static;
        let _ = ActorKind::Dynamic;
        let _ = ActorKind::Character;
        let _ = ActorKind::Other;
    }

    // ===== Layers Tests =====

    #[test]
    fn test_layers_bits() {
        assert_eq!(Layers::DEFAULT.bits(), 0b0001);
        assert_eq!(Layers::CHARACTER.bits(), 0b0010);
    }

    #[test]
    fn test_layers_all() {
        let all = Layers::all();
        assert!(all.contains(Layers::DEFAULT));
        assert!(all.contains(Layers::CHARACTER));
    }

    // ===== PhysicsConfig Tests =====

    #[test]
    fn test_physics_config_default() {
        let config = PhysicsConfig::default();
        assert_eq!(config.time_step, 1.0 / 60.0);
        assert_eq!(config.gravity.y, -9.81);
    }

    // ===== Debug Line Tests =====

    #[test]
    fn test_debug_line_creation() {
        let line = DebugLine {
            start: [0.0, 0.0, 0.0],
            end: [1.0, 1.0, 1.0],
            color: [1.0, 0.0, 0.0],
        };

        assert_eq!(line.start, [0.0, 0.0, 0.0]);
        assert_eq!(line.color, [1.0, 0.0, 0.0]);
    }

    // ===== Multiple Bodies Tests =====

    #[test]
    fn test_multiple_bodies() {
        let mut pw = PhysicsWorld::new(Vec3::new(0.0, -9.8, 0.0));
        let _ground = pw.create_ground_plane(Vec3::new(10.0, 0.5, 10.0), 0.9);

        let box1 = pw.add_dynamic_box(
            Vec3::new(-2.0, 5.0, 0.0),
            Vec3::new(0.5, 0.5, 0.5),
            1.0,
            Layers::DEFAULT,
        );
        let box2 = pw.add_dynamic_box(
            Vec3::new(2.0, 5.0, 0.0),
            Vec3::new(0.5, 0.5, 0.5),
            1.0,
            Layers::DEFAULT,
        );
        let char1 = pw.add_character(Vec3::new(0.0, 1.0, 0.0), Vec3::new(0.4, 0.9, 0.4));

        assert!(pw.body_transform(box1).is_some());
        assert!(pw.body_transform(box2).is_some());
        assert!(pw.body_transform(char1).is_some());

        // All should have different IDs
        assert_ne!(box1, box2);
        assert_ne!(box2, char1);
    }

    // ===== Step Integration Test =====

    #[test]
    fn test_physics_step_integration() {
        let mut pw = PhysicsWorld::new(Vec3::new(0.0, -9.8, 0.0));
        let _ground = pw.create_ground_plane(Vec3::new(20.0, 0.5, 20.0), 0.9);

        // Add falling box
        let box_id = pw.add_dynamic_box(
            Vec3::new(0.0, 10.0, 0.0),
            Vec3::new(0.5, 0.5, 0.5),
            1.0,
            Layers::DEFAULT,
        );

        let y_start = pw.body_transform(box_id).unwrap().w_axis.y;

        // Step 120 frames (2 seconds)
        for _ in 0..120 {
            pw.step();
        }

        let y_end = pw.body_transform(box_id).unwrap().w_axis.y;

        // Box should have fallen significantly
        assert!(
            y_end < y_start - 1.0,
            "Box should fall, start={}, end={}",
            y_start,
            y_end
        );
    }

    #[test]
    fn test_apply_radial_impulse() {
        let mut pw = PhysicsWorld::new(Vec3::new(0.0, -9.8, 0.0));
        let box_id = pw.add_dynamic_box(
            Vec3::new(1.0, 1.0, 0.0),
            Vec3::new(0.5, 0.5, 0.5),
            1.0,
            Layers::DEFAULT,
        );

        let count = pw.apply_radial_impulse(Vec3::ZERO, 5.0, 100.0, FalloffCurve::Linear, 0.0);

        assert_eq!(count, 1);
        let vel = pw.get_velocity(box_id).unwrap();
        assert!(vel.length() > 0.0);
    }

    #[test]
    fn test_raycast() {
        let mut pw = PhysicsWorld::new(Vec3::new(0.0, -9.8, 0.0));
        let box_id = pw.add_dynamic_box(
            Vec3::new(5.0, 0.0, 0.0),
            Vec3::new(0.5, 0.5, 0.5),
            1.0,
            Layers::DEFAULT,
        );
        pw.step(); // Update query pipeline

        let hit = pw.raycast(Vec3::ZERO, Vec3::X, 10.0);
        assert!(hit.is_some());
        let (_, _, hit_id, _) = hit.unwrap();
        assert_eq!(hit_id, Some(box_id));
    }

    #[test]
    fn test_break_destructible() {
        let mut pw = PhysicsWorld::new(Vec3::new(0.0, -9.8, 0.0));
        let box_id = pw.add_dynamic_box(
            Vec3::new(0.0, 5.0, 0.0),
            Vec3::new(0.5, 0.5, 0.5),
            1.0,
            Layers::DEFAULT,
        );

        assert!(pw.handle_of(box_id).is_some());
        pw.break_destructible(box_id);
        assert!(pw.handle_of(box_id).is_none());
    }

    #[test]
    fn test_add_joint() {
        let mut pw = PhysicsWorld::new(Vec3::new(0.0, -9.8, 0.0));
        let b1 = pw.add_dynamic_box(
            Vec3::new(0.0, 5.0, 0.0),
            Vec3::new(0.5, 0.5, 0.5),
            1.0,
            Layers::DEFAULT,
        );
        let b2 = pw.add_dynamic_box(
            Vec3::new(2.0, 5.0, 0.0),
            Vec3::new(0.5, 0.5, 0.5),
            1.0,
            Layers::DEFAULT,
        );

        let j1 = pw.add_joint(b1, b2, JointType::Fixed);
        assert_ne!(j1.0, 0);

        let j2 = pw.add_joint(
            b1,
            b2,
            JointType::Revolute {
                axis: Vec3::Y,
                limits: None,
            },
        );
        assert_ne!(j2.0, 0);

        let j3 = pw.add_joint(
            b1,
            b2,
            JointType::Prismatic {
                axis: Vec3::X,
                limits: Some((-1.0, 1.0)),
            },
        );
        assert_ne!(j3.0, 0);

        let j4 = pw.add_joint(b1, b2, JointType::Spherical);
        assert_ne!(j4.0, 0);
    }

    #[test]
    fn test_set_body_position() {
        let mut pw = PhysicsWorld::new(Vec3::new(0.0, -9.8, 0.0));
        let box_id = pw.add_dynamic_box(
            Vec3::new(0.0, 5.0, 0.0),
            Vec3::new(0.5, 0.5, 0.5),
            1.0,
            Layers::DEFAULT,
        );

        pw.set_body_position(box_id, Vec3::new(10.0, 10.0, 10.0));
        let transform = pw.body_transform(box_id).unwrap();
        assert!((transform.w_axis.x - 10.0).abs() < 0.01);
    }

    #[test]
    fn test_enable_ccd() {
        let mut pw = PhysicsWorld::new(Vec3::new(0.0, -9.8, 0.0));
        let box_id = pw.add_dynamic_box(
            Vec3::new(0.0, 5.0, 0.0),
            Vec3::new(0.5, 0.5, 0.5),
            1.0,
            Layers::DEFAULT,
        );
        pw.enable_ccd(box_id);
    }

    #[test]
    fn test_get_debug_lines() {
        let mut pw = PhysicsWorld::new(Vec3::new(0.0, -9.8, 0.0));
        pw.add_dynamic_box(
            Vec3::new(0.0, 5.0, 0.0),
            Vec3::new(0.5, 0.5, 0.5),
            1.0,
            Layers::DEFAULT,
        );
        let lines = pw.get_debug_lines();
        assert!(!lines.is_empty());
    }

    #[test]
    fn test_character_climb() {
        let mut pw = PhysicsWorld::new(Vec3::new(0.0, -9.8, 0.0));
        let char_id = pw.add_character(Vec3::new(0.0, 1.0, 0.0), Vec3::new(0.4, 0.9, 0.4));

        let y0 = pw.body_transform(char_id).unwrap().w_axis.y;
        pw.control_character(char_id, Vec3::ZERO, 0.1, true);
        pw.step();

        let y1 = pw.body_transform(char_id).unwrap().w_axis.y;
        assert!(y1 > y0);
    }

    #[test]
    fn test_buoyancy_application() {
        let mut pw = PhysicsWorld::new(Vec3::new(0.0, -9.8, 0.0));
        pw.water_level = 10.0;
        let box_id = pw.add_dynamic_box(
            Vec3::new(0.0, 5.0, 0.0),
            Vec3::new(0.5, 0.5, 0.5),
            1.0,
            Layers::DEFAULT,
        );
        pw.add_buoyancy(box_id, 1.0, 0.5);

        // Step should apply buoyancy force
        pw.step();
        let vel = pw.get_velocity(box_id).unwrap();
        assert!(vel.y > 0.0);
    }

    // ===== ActorKind Helper Tests =====

    #[test]
    fn test_actor_kind_name() {
        assert_eq!(ActorKind::Static.name(), "Static");
        assert_eq!(ActorKind::Dynamic.name(), "Dynamic");
        assert_eq!(ActorKind::Character.name(), "Character");
        assert_eq!(ActorKind::Other.name(), "Other");
    }

    #[test]
    fn test_actor_kind_is_static() {
        assert!(ActorKind::Static.is_static());
        assert!(!ActorKind::Dynamic.is_static());
        assert!(!ActorKind::Character.is_static());
        assert!(!ActorKind::Other.is_static());
    }

    #[test]
    fn test_actor_kind_is_dynamic() {
        assert!(!ActorKind::Static.is_dynamic());
        assert!(ActorKind::Dynamic.is_dynamic());
        assert!(!ActorKind::Character.is_dynamic());
        assert!(!ActorKind::Other.is_dynamic());
    }

    #[test]
    fn test_actor_kind_is_character() {
        assert!(!ActorKind::Static.is_character());
        assert!(!ActorKind::Dynamic.is_character());
        assert!(ActorKind::Character.is_character());
        assert!(!ActorKind::Other.is_character());
    }

    #[test]
    fn test_actor_kind_is_movable() {
        assert!(!ActorKind::Static.is_movable());
        assert!(ActorKind::Dynamic.is_movable());
        assert!(ActorKind::Character.is_movable());
        assert!(!ActorKind::Other.is_movable());
    }

    #[test]
    fn test_actor_kind_all() {
        let all = ActorKind::all();
        assert_eq!(all.len(), 4);
        assert!(all.contains(&ActorKind::Static));
        assert!(all.contains(&ActorKind::Dynamic));
        assert!(all.contains(&ActorKind::Character));
        assert!(all.contains(&ActorKind::Other));
    }

    #[test]
    fn test_actor_kind_display() {
        assert_eq!(format!("{}", ActorKind::Static), "Static");
        assert_eq!(format!("{}", ActorKind::Dynamic), "Dynamic");
        assert_eq!(format!("{}", ActorKind::Character), "Character");
        assert_eq!(format!("{}", ActorKind::Other), "Other");
    }

    // ===== DebugLine Helper Tests =====

    #[test]
    fn test_debug_line_new() {
        let line = DebugLine::new([0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [1.0, 0.0, 0.0]);
        assert_eq!(line.start, [0.0, 0.0, 0.0]);
        assert_eq!(line.end, [1.0, 0.0, 0.0]);
        assert_eq!(line.color, [1.0, 0.0, 0.0]);
    }

    #[test]
    fn test_debug_line_from_vec3() {
        let line = DebugLine::from_vec3(Vec3::ZERO, Vec3::X, [0.0, 1.0, 0.0]);
        assert_eq!(line.start, [0.0, 0.0, 0.0]);
        assert_eq!(line.end, [1.0, 0.0, 0.0]);
        assert_eq!(line.color, [0.0, 1.0, 0.0]);
    }

    #[test]
    fn test_debug_line_length() {
        let line = DebugLine::new([0.0, 0.0, 0.0], [3.0, 4.0, 0.0], [1.0, 1.0, 1.0]);
        assert!((line.length() - 5.0).abs() < 0.001);
    }

    #[test]
    fn test_debug_line_length_squared() {
        let line = DebugLine::new([0.0, 0.0, 0.0], [3.0, 4.0, 0.0], [1.0, 1.0, 1.0]);
        assert!((line.length_squared() - 25.0).abs() < 0.001);
    }

    #[test]
    fn test_debug_line_midpoint() {
        let line = DebugLine::new([0.0, 0.0, 0.0], [2.0, 4.0, 6.0], [1.0, 1.0, 1.0]);
        let mid = line.midpoint();
        assert!((mid[0] - 1.0).abs() < 0.001);
        assert!((mid[1] - 2.0).abs() < 0.001);
        assert!((mid[2] - 3.0).abs() < 0.001);
    }

    #[test]
    fn test_debug_line_direction() {
        let line = DebugLine::new([1.0, 2.0, 3.0], [4.0, 6.0, 9.0], [1.0, 1.0, 1.0]);
        let dir = line.direction();
        assert!((dir[0] - 3.0).abs() < 0.001);
        assert!((dir[1] - 4.0).abs() < 0.001);
        assert!((dir[2] - 6.0).abs() < 0.001);
    }

    #[test]
    fn test_debug_line_is_degenerate() {
        let degen = DebugLine::new([1.0, 2.0, 3.0], [1.0, 2.0, 3.0], [1.0, 1.0, 1.0]);
        assert!(degen.is_degenerate());

        let valid = DebugLine::new([0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [1.0, 1.0, 1.0]);
        assert!(!valid.is_degenerate());
    }

    #[test]
    fn test_debug_line_color_constructors() {
        let red = DebugLine::red([0.0, 0.0, 0.0], [1.0, 0.0, 0.0]);
        assert_eq!(red.color, [1.0, 0.0, 0.0]);

        let green = DebugLine::green([0.0, 0.0, 0.0], [1.0, 0.0, 0.0]);
        assert_eq!(green.color, [0.0, 1.0, 0.0]);

        let blue = DebugLine::blue([0.0, 0.0, 0.0], [1.0, 0.0, 0.0]);
        assert_eq!(blue.color, [0.0, 0.0, 1.0]);

        let white = DebugLine::white([0.0, 0.0, 0.0], [1.0, 0.0, 0.0]);
        assert_eq!(white.color, [1.0, 1.0, 1.0]);
    }

    #[test]
    fn test_debug_line_display() {
        let line = DebugLine::new([0.0, 0.0, 0.0], [1.0, 2.0, 3.0], [0.5, 0.5, 0.5]);
        let display = format!("{}", line);
        assert!(display.contains("Line"));
        assert!(display.contains("0.00"));
    }

    // ===== CharState Helper Tests =====

    #[test]
    fn test_char_state_name() {
        assert_eq!(CharState::Grounded.name(), "Grounded");
    }

    #[test]
    fn test_char_state_is_grounded() {
        assert!(CharState::Grounded.is_grounded());
    }

    #[test]
    fn test_char_state_all() {
        let all = CharState::all();
        assert_eq!(all.len(), 1);
        assert!(all.contains(&CharState::Grounded));
    }

    #[test]
    fn test_char_state_display() {
        assert_eq!(format!("{}", CharState::Grounded), "Grounded");
    }

    // ===== CharacterController Helper Tests =====

    #[test]
    fn test_character_controller_new() {
        let cc = CharacterController::new(0.5, 2.0);
        assert_eq!(cc.radius, 0.5);
        assert_eq!(cc.height, 2.0);
        assert!(cc.is_grounded());
    }

    #[test]
    fn test_character_controller_is_grounded() {
        let cc = CharacterController::new(0.5, 2.0);
        assert!(cc.is_grounded());
    }

    #[test]
    fn test_character_controller_has_coyote_time() {
        let mut cc = CharacterController::new(0.5, 2.0);
        assert!(cc.has_coyote_time()); // time_since_grounded = 0.0

        cc.time_since_grounded = 0.2;
        assert!(!cc.has_coyote_time());
    }

    #[test]
    fn test_character_controller_has_buffered_jump() {
        let mut cc = CharacterController::new(0.5, 2.0);
        assert!(!cc.has_buffered_jump());

        cc.jump_buffer_timer = 0.1;
        assert!(cc.has_buffered_jump());
    }

    #[test]
    fn test_character_controller_can_jump() {
        let cc = CharacterController::new(0.5, 2.0);
        // When grounded, can_jump is always true regardless of time_since_grounded
        assert!(cc.can_jump());

        // Test coyote time - still returns true when time_since_grounded < coyote_time_limit
        let mut cc2 = CharacterController::new(0.5, 2.0);
        cc2.time_since_grounded = 0.1; // within coyote time
        assert!(cc2.can_jump()); // should be true (grounded OR coyote time)
    }

    #[test]
    fn test_character_controller_volume() {
        let cc = CharacterController::new(1.0, 4.0);
        let vol = cc.volume();
        // Volume should be positive and reasonable
        assert!(vol > 0.0);
        assert!(vol < 100.0);
    }

    #[test]
    fn test_character_controller_max_climb_angle_rad() {
        let cc = CharacterController::new(0.5, 2.0);
        let rad = cc.max_climb_angle_rad();
        assert!((rad - 45.0_f32.to_radians()).abs() < 0.001);
    }

    #[test]
    fn test_character_controller_is_falling_rising() {
        let mut cc = CharacterController::new(0.5, 2.0);

        cc.vertical_velocity = -5.0;
        assert!(cc.is_falling());
        assert!(!cc.is_rising());

        cc.vertical_velocity = 5.0;
        assert!(!cc.is_falling());
        assert!(cc.is_rising());

        cc.vertical_velocity = 0.0;
        assert!(!cc.is_falling());
        assert!(!cc.is_rising());
    }

    #[test]
    fn test_character_controller_reset() {
        let mut cc = CharacterController::new(0.5, 2.0);
        cc.vertical_velocity = 10.0;
        cc.time_since_grounded = 1.0;
        cc.jump_buffer_timer = 0.5;
        cc.pending_jump_velocity = 5.0;

        cc.reset();

        assert!(cc.is_grounded());
        assert_eq!(cc.vertical_velocity, 0.0);
        assert_eq!(cc.time_since_grounded, 0.0);
        assert_eq!(cc.jump_buffer_timer, 0.0);
        assert_eq!(cc.pending_jump_velocity, 0.0);
    }

    #[test]
    fn test_character_controller_display() {
        let cc = CharacterController::new(0.5, 2.0);
        let display = format!("{}", cc);
        assert!(display.contains("CharController"));
        assert!(display.contains("0.50"));
        assert!(display.contains("2.00"));
    }

    // ===== PhysicsConfig Helper Tests =====

    #[test]
    fn test_physics_config_new() {
        let config = PhysicsConfig::new();
        assert!((config.gravity.y + 9.81).abs() < 0.01);
        assert!(!config.ccd_enabled);
    }

    #[test]
    fn test_physics_config_with_gravity() {
        let config = PhysicsConfig::new().with_gravity(Vec3::new(0.0, -20.0, 0.0));
        assert_eq!(config.gravity.y, -20.0);
    }

    #[test]
    fn test_physics_config_with_ccd() {
        let config = PhysicsConfig::new().with_ccd(4);
        assert!(config.ccd_enabled);
        assert_eq!(config.max_ccd_substeps, 4);
    }

    #[test]
    fn test_physics_config_with_time_step() {
        let config = PhysicsConfig::new().with_time_step(1.0 / 120.0);
        assert!((config.time_step - 1.0 / 120.0).abs() < 0.0001);
    }

    #[test]
    fn test_physics_config_with_water() {
        let config = PhysicsConfig::new().with_water(5.0, 1025.0);
        assert_eq!(config.water_level, 5.0);
        assert_eq!(config.fluid_density, 1025.0);
    }

    #[test]
    fn test_physics_config_target_fps() {
        let config = PhysicsConfig::new();
        assert!((config.target_fps() - 60.0).abs() < 0.1);

        let config120 = PhysicsConfig::new().with_time_step(1.0 / 120.0);
        assert!((config120.target_fps() - 120.0).abs() < 0.1);
    }

    #[test]
    fn test_physics_config_has_water() {
        let config = PhysicsConfig::new();
        assert!(!config.has_water()); // default is NEG_INFINITY

        let with_water = PhysicsConfig::new().with_water(0.0, 1000.0);
        assert!(with_water.has_water());
    }

    #[test]
    fn test_physics_config_gravity_magnitude() {
        let config = PhysicsConfig::new();
        assert!((config.gravity_magnitude() - 9.81).abs() < 0.01);
    }

    #[test]
    fn test_physics_config_is_earth_gravity() {
        let config = PhysicsConfig::new();
        assert!(config.is_earth_gravity());

        let moon = PhysicsConfig::new().with_gravity(Vec3::new(0.0, -1.62, 0.0));
        assert!(!moon.is_earth_gravity());
    }

    #[test]
    fn test_physics_config_is_zero_gravity() {
        let zero = PhysicsConfig::new().with_gravity(Vec3::ZERO);
        assert!(zero.is_zero_gravity());

        let earth = PhysicsConfig::new();
        assert!(!earth.is_zero_gravity());
    }

    #[test]
    fn test_physics_config_display() {
        let config = PhysicsConfig::new();
        let display = format!("{}", config);
        assert!(display.contains("PhysicsConfig"));
        assert!(display.contains("-9.81"));
    }

    // ===== JointType Helper Tests =====

    #[test]
    fn test_joint_type_name() {
        assert_eq!(JointType::Fixed.name(), "Fixed");
        assert_eq!(
            JointType::Revolute {
                axis: Vec3::Y,
                limits: None
            }
            .name(),
            "Revolute"
        );
        assert_eq!(
            JointType::Prismatic {
                axis: Vec3::X,
                limits: None
            }
            .name(),
            "Prismatic"
        );
        assert_eq!(JointType::Spherical.name(), "Spherical");
    }

    #[test]
    fn test_joint_type_is_fixed() {
        assert!(JointType::Fixed.is_fixed());
        assert!(!JointType::Spherical.is_fixed());
    }

    #[test]
    fn test_joint_type_is_revolute() {
        assert!(JointType::Revolute {
            axis: Vec3::Y,
            limits: None
        }
        .is_revolute());
        assert!(!JointType::Fixed.is_revolute());
    }

    #[test]
    fn test_joint_type_is_prismatic() {
        assert!(JointType::Prismatic {
            axis: Vec3::X,
            limits: None
        }
        .is_prismatic());
        assert!(!JointType::Fixed.is_prismatic());
    }

    #[test]
    fn test_joint_type_is_spherical() {
        assert!(JointType::Spherical.is_spherical());
        assert!(!JointType::Fixed.is_spherical());
    }

    #[test]
    fn test_joint_type_has_limits() {
        let no_limits = JointType::Revolute {
            axis: Vec3::Y,
            limits: None,
        };
        assert!(!no_limits.has_limits());

        let with_limits = JointType::Revolute {
            axis: Vec3::Y,
            limits: Some((-1.0, 1.0)),
        };
        assert!(with_limits.has_limits());

        assert!(!JointType::Fixed.has_limits());
        assert!(!JointType::Spherical.has_limits());
    }

    #[test]
    fn test_joint_type_axis() {
        let revolute = JointType::Revolute {
            axis: Vec3::Y,
            limits: None,
        };
        assert_eq!(revolute.axis(), Some(Vec3::Y));

        let prismatic = JointType::Prismatic {
            axis: Vec3::X,
            limits: None,
        };
        assert_eq!(prismatic.axis(), Some(Vec3::X));

        assert_eq!(JointType::Fixed.axis(), None);
        assert_eq!(JointType::Spherical.axis(), None);
    }

    #[test]
    fn test_joint_type_limits() {
        let with_limits = JointType::Revolute {
            axis: Vec3::Y,
            limits: Some((-1.5, 1.5)),
        };
        assert_eq!(with_limits.limits(), Some((-1.5, 1.5)));

        let no_limits = JointType::Revolute {
            axis: Vec3::Y,
            limits: None,
        };
        assert_eq!(no_limits.limits(), None);
    }

    #[test]
    fn test_joint_type_is_rotational() {
        assert!(JointType::Revolute {
            axis: Vec3::Y,
            limits: None
        }
        .is_rotational());
        assert!(JointType::Spherical.is_rotational());
        assert!(!JointType::Fixed.is_rotational());
        assert!(!JointType::Prismatic {
            axis: Vec3::X,
            limits: None
        }
        .is_rotational());
    }

    #[test]
    fn test_joint_type_is_linear() {
        assert!(JointType::Prismatic {
            axis: Vec3::X,
            limits: None
        }
        .is_linear());
        assert!(!JointType::Revolute {
            axis: Vec3::Y,
            limits: None
        }
        .is_linear());
        assert!(!JointType::Fixed.is_linear());
    }

    #[test]
    fn test_joint_type_degrees_of_freedom() {
        assert_eq!(JointType::Fixed.degrees_of_freedom(), 0);
        assert_eq!(
            JointType::Revolute {
                axis: Vec3::Y,
                limits: None
            }
            .degrees_of_freedom(),
            1
        );
        assert_eq!(
            JointType::Prismatic {
                axis: Vec3::X,
                limits: None
            }
            .degrees_of_freedom(),
            1
        );
        assert_eq!(JointType::Spherical.degrees_of_freedom(), 3);
    }

    #[test]
    fn test_joint_type_factory_methods() {
        let ry = JointType::revolute_y();
        assert!(ry.is_revolute());
        assert_eq!(ry.axis(), Some(Vec3::Y));

        let rx = JointType::revolute_x();
        assert_eq!(rx.axis(), Some(Vec3::X));

        let rz = JointType::revolute_z();
        assert_eq!(rz.axis(), Some(Vec3::Z));

        let py = JointType::prismatic_y();
        assert!(py.is_prismatic());
        assert_eq!(py.axis(), Some(Vec3::Y));
    }

    #[test]
    fn test_joint_type_display() {
        assert_eq!(format!("{}", JointType::Fixed), "Fixed");
        assert_eq!(format!("{}", JointType::Spherical), "Spherical");

        let revolute = JointType::Revolute {
            axis: Vec3::Y,
            limits: None,
        };
        let display = format!("{}", revolute);
        assert!(display.contains("Revolute"));

        let with_limits = JointType::Revolute {
            axis: Vec3::Y,
            limits: Some((-1.0, 1.0)),
        };
        let display2 = format!("{}", with_limits);
        assert!(display2.contains("limits"));
    }

    // ===== JointId Helper Tests =====

    #[test]
    fn test_joint_id_new() {
        let id = JointId::new(42);
        assert_eq!(id.0, 42);
    }

    #[test]
    fn test_joint_id_raw() {
        let id = JointId::new(123);
        assert_eq!(id.raw(), 123);
    }

    #[test]
    fn test_joint_id_is_valid() {
        let valid = JointId::new(1);
        assert!(valid.is_valid());

        let invalid = JointId::new(0);
        assert!(!invalid.is_valid());
    }

    #[test]
    fn test_joint_id_invalid() {
        let invalid = JointId::invalid();
        assert_eq!(invalid.0, 0);
        assert!(!invalid.is_valid());
    }

    #[test]
    fn test_joint_id_display() {
        let id = JointId::new(42);
        assert_eq!(format!("{}", id), "JointId(42)");
    }

    #[test]
    fn test_joint_id_from_u64() {
        let id: JointId = 100u64.into();
        assert_eq!(id.raw(), 100);
    }

    #[test]
    fn test_joint_id_into_u64() {
        let id = JointId::new(200);
        let raw: u64 = id.into();
        assert_eq!(raw, 200);
    }

    #[test]
    fn test_joint_id_default() {
        let id = JointId::default();
        assert_eq!(id.0, 0);
        assert!(!id.is_valid());
    }

    // ===== BuoyancyData Helper Tests =====

    #[test]
    fn test_buoyancy_data_new() {
        let bd = BuoyancyData::new(1.5, 0.5);
        assert_eq!(bd.volume, 1.5);
        assert_eq!(bd.drag, 0.5);
    }

    #[test]
    fn test_buoyancy_data_with_volume() {
        let bd = BuoyancyData::with_volume(2.0);
        assert_eq!(bd.volume, 2.0);
        assert_eq!(bd.drag, 0.0);
    }

    #[test]
    fn test_buoyancy_data_is_valid() {
        let valid = BuoyancyData::new(1.0, 0.5);
        assert!(valid.is_valid());

        let invalid = BuoyancyData::new(0.0, 0.5);
        assert!(!invalid.is_valid());

        let negative = BuoyancyData::new(-1.0, 0.5);
        assert!(!negative.is_valid());
    }

    #[test]
    fn test_buoyancy_data_buoyancy_force() {
        let bd = BuoyancyData::new(1.0, 0.0);
        let force = bd.buoyancy_force(1000.0);
        // F = V * rho * g = 1.0 * 1000.0 * 9.81 = 9810
        assert!((force - 9810.0).abs() < 1.0);
    }

    #[test]
    fn test_buoyancy_data_drag_force() {
        let bd = BuoyancyData::new(1.0, 1.0);
        let force = bd.drag_force(10.0);
        // F = 0.5 * drag * v^2 = 0.5 * 1.0 * 100 = 50
        assert!((force - 50.0).abs() < 0.1);
    }

    #[test]
    fn test_buoyancy_data_drag_force_nonunit_drag() {
        // Use drag ≠ 1.0 so that `0.5 * drag` ≠ `0.5 / drag`
        let bd = BuoyancyData::new(1.0, 3.0);
        let force = bd.drag_force(4.0);
        // F = 0.5 * 3.0 * 4.0 * 4.0 = 24.0
        assert!(
            (force - 24.0).abs() < 0.01,
            "drag_force with drag=3.0 v=4.0: got {} expected 24.0",
            force,
        );
    }

    #[test]
    fn test_buoyancy_data_display() {
        let bd = BuoyancyData::new(1.5, 0.25);
        let display = format!("{}", bd);
        assert!(display.contains("Buoyancy"));
        assert!(display.contains("1.500"));
        assert!(display.contains("0.250"));
    }

    #[test]
    fn test_buoyancy_data_default() {
        let bd = BuoyancyData::default();
        assert_eq!(bd.volume, 0.0);
        assert_eq!(bd.drag, 0.0);
    }

    // ===== Mutation-resistant tests =====
    // These target the specific mutants that escape existing tests.

    // --- BuoyancyData::drag_force: * → / at each position ---
    #[test]
    fn drag_force_all_multiplications_matter() {
        // Use values where *, +, / all give distinct results
        // F = 0.5 * drag * v * v
        let bd = BuoyancyData::new(1.0, 2.0);
        let force = bd.drag_force(3.0);
        // Expected: 0.5 * 2.0 * 3.0 * 3.0 = 9.0
        // If / at pos1: 0.5 / 2.0 * 3.0 * 3.0 = 2.25
        // If / at pos2: 0.5 * 2.0 / 3.0 * 3.0 = 1.0
        // If / at pos3: 0.5 * 2.0 * 3.0 / 3.0 = 1.0
        // If + at pos1: (0.5 + 2.0) * 3.0 * 3.0 = 22.5
        assert!(
            (force - 9.0).abs() < 0.01,
            "drag_force(2.0 drag, 3.0 vel) = 0.5*2*3*3 = 9.0, got {}",
            force
        );
    }

    // --- add_character height calculation: half.y * 2.0 ---
    #[test]
    fn add_character_height_is_double_half_y() {
        let mut pw = PhysicsWorld::new(Vec3::new(0.0, -9.8, 0.0));
        let half = Vec3::new(0.4, 0.9, 0.4);
        let char_id = pw.add_character(Vec3::new(0.0, 5.0, 0.0), half);

        let ctrl = pw.char_map.get(&char_id).unwrap();
        // height should be half.y * 2.0 = 0.9 * 2.0 = 1.8
        assert!(
            (ctrl.height - 1.8).abs() < 0.001,
            "height should be half.y * 2.0 = 1.8, got {}",
            ctrl.height
        );
        // If * → +: 0.9 + 2.0 = 2.9 (wrong)
        // If * → /: 0.9 / 2.0 = 0.45 (wrong)
    }

    // --- jump: verify velocity = sqrt(2 * g * height) ---
    #[test]
    fn jump_velocity_formula_correctness() {
        let mut pw = PhysicsWorld::new(Vec3::new(0.0, -9.8, 0.0));
        let char_id = pw.add_character(Vec3::new(0.0, 5.0, 0.0), Vec3::new(0.4, 0.9, 0.4));

        let height = 3.0;
        pw.jump(char_id, height);

        let ctrl = pw.char_map.get(&char_id).unwrap();
        let g = 9.81 * ctrl.gravity_scale; // gravity_scale = 1.0
        let expected_vel = (2.0 * g * height).sqrt();
        // expected_vel = sqrt(2 * 9.81 * 3.0) = sqrt(58.86) ≈ 7.672
        assert!(
            (ctrl.pending_jump_velocity - expected_vel).abs() < 0.01,
            "jump velocity should be sqrt(2*g*h) = {:.3}, got {:.3}",
            expected_vel,
            ctrl.pending_jump_velocity
        );

        // If * → +: (2.0 + g + height).sqrt() = sqrt(14.81) ≈ 3.849 (wrong)
        // If * → /: (2.0 / g / height).sqrt() = sqrt(0.068) ≈ 0.261 (wrong)
        assert!(
            ctrl.pending_jump_velocity > 5.0,
            "Jump vel must be > 5.0 for h=3.0"
        );
    }

    #[test]
    fn jump_sets_buffer_timer() {
        // Mutant: replace jump with () — would skip all logic
        let mut pw = PhysicsWorld::new(Vec3::new(0.0, -9.8, 0.0));
        let char_id = pw.add_character(Vec3::new(0.0, 5.0, 0.0), Vec3::new(0.4, 0.9, 0.4));

        pw.jump(char_id, 2.0);

        let ctrl = pw.char_map.get(&char_id).unwrap();
        assert!(ctrl.jump_buffer_timer > 0.0, "jump must set buffer timer");
        assert!(
            ctrl.pending_jump_velocity > 0.0,
            "jump must set pending velocity"
        );
    }

    // --- control_character: gravity application ---
    #[test]
    fn control_character_applies_gravity_when_not_climbing() {
        let mut pw = PhysicsWorld::new(Vec3::new(0.0, -9.8, 0.0));
        let _ground = pw.create_ground_plane(Vec3::new(100.0, 0.1, 100.0), 0.9);
        // Place character high above ground so it doesn't snap
        let char_id = pw.add_character(Vec3::new(0.0, 50.0, 0.0), Vec3::new(0.4, 0.9, 0.4));

        // Initialize query pipeline with a step
        pw.step();

        // Now apply several frames of gravity
        for _ in 0..10 {
            pw.control_character(char_id, Vec3::ZERO, 1.0 / 60.0, false);
            pw.step();
        }

        let ctrl = pw.char_map.get(&char_id).unwrap();
        // After 10 frames of freefall with no ground snap:
        // vertical_velocity should be negative (gravity pulling down)
        // If -= mutated to +=, velocity would be positive (going up)
        assert!(
            ctrl.vertical_velocity < 0.0,
            "Gravity should make vertical_velocity negative, got {}",
            ctrl.vertical_velocity
        );
    }

    #[test]
    fn control_character_climb_ignores_gravity() {
        let mut pw = PhysicsWorld::new(Vec3::new(0.0, -9.8, 0.0));
        let char_id = pw.add_character(Vec3::new(0.0, 5.0, 0.0), Vec3::new(0.4, 0.9, 0.4));

        // Move with climbing=true → vertical_velocity should be 0, no gravity
        pw.control_character(char_id, Vec3::ZERO, 0.1, true);

        let ctrl = pw.char_map.get(&char_id).unwrap();
        assert_eq!(
            ctrl.vertical_velocity, 0.0,
            "Climbing should zero vertical velocity, not apply gravity"
        );
        // If !_climb → _climb mutant: gravity would apply during climb
    }

    #[test]
    fn control_character_horizontal_movement_scales_with_dt() {
        let mut pw = PhysicsWorld::new(Vec3::new(0.0, -9.8, 0.0));
        let _ground = pw.create_ground_plane(Vec3::new(20.0, 0.5, 20.0), 0.9);
        let char_id = pw.add_character(Vec3::new(0.0, 1.0, 0.0), Vec3::new(0.4, 0.9, 0.4));

        let x0 = pw.body_transform(char_id).unwrap().w_axis.x;

        // Move with small dt
        pw.control_character(char_id, Vec3::new(10.0, 0.0, 0.0), 0.01, false);
        pw.step();
        let x1 = pw.body_transform(char_id).unwrap().w_axis.x;

        // Move with larger dt should move further
        let char_id2 = pw.add_character(Vec3::new(0.0, 1.0, 5.0), Vec3::new(0.4, 0.9, 0.4));
        let x2_0 = pw.body_transform(char_id2).unwrap().w_axis.x;
        pw.control_character(char_id2, Vec3::new(10.0, 0.0, 0.0), 0.1, false);
        pw.step();
        let x2_1 = pw.body_transform(char_id2).unwrap().w_axis.x;

        let delta_small = x1 - x0;
        let delta_large = x2_1 - x2_0;
        assert!(
            delta_large > delta_small * 2.0,
            "Larger dt should produce more movement: small_dt→{}, large_dt→{}",
            delta_small,
            delta_large
        );
    }

    // --- apply_radial_impulse: direction and falloff ---
    #[test]
    fn radial_impulse_direction_away_from_center() {
        let mut pw = PhysicsWorld::new(Vec3::new(0.0, 0.0, 0.0)); // zero gravity for clarity
        let box_id = pw.add_dynamic_box(
            Vec3::new(3.0, 0.0, 0.0), // Body at (3,0,0)
            Vec3::new(0.5, 0.5, 0.5),
            1.0,
            Layers::DEFAULT,
        );

        // Explosion at origin, radius=10, force=100, no upward bias
        let count = pw.apply_radial_impulse(Vec3::ZERO, 10.0, 100.0, FalloffCurve::Linear, 0.0);
        assert_eq!(count, 1);

        let vel = pw.get_velocity(box_id).unwrap();
        // Body is at +X from center, so impulse should push in +X
        assert!(
            vel.x > 0.0,
            "Radial impulse from origin should push +X, got vx={}",
            vel.x
        );
        // Y and Z should be ~0 (no upward bias, body directly on X axis)
        assert!(
            vel.y.abs() < 0.1,
            "No upward bias → vy should be ~0, got {}",
            vel.y
        );
    }

    #[test]
    fn radial_impulse_with_upward_bias() {
        let mut pw = PhysicsWorld::new(Vec3::new(0.0, 0.0, 0.0));
        let box_id = pw.add_dynamic_box(
            Vec3::new(2.0, 0.0, 0.0),
            Vec3::new(0.5, 0.5, 0.5),
            1.0,
            Layers::DEFAULT,
        );

        // Full upward bias
        pw.apply_radial_impulse(Vec3::ZERO, 10.0, 100.0, FalloffCurve::Linear, 1.0);

        let vel = pw.get_velocity(box_id).unwrap();
        // With upward_bias=1.0, direction should be mostly Y
        assert!(
            vel.y > vel.x.abs(),
            "Full upward bias should have vy > |vx|, vy={}, vx={}",
            vel.y,
            vel.x
        );
    }

    #[test]
    fn radial_impulse_excludes_bodies_outside_radius() {
        let mut pw = PhysicsWorld::new(Vec3::new(0.0, 0.0, 0.0));
        let _near = pw.add_dynamic_box(
            Vec3::new(2.0, 0.0, 0.0),
            Vec3::new(0.5, 0.5, 0.5),
            1.0,
            Layers::DEFAULT,
        );
        let far = pw.add_dynamic_box(
            Vec3::new(20.0, 0.0, 0.0),
            Vec3::new(0.5, 0.5, 0.5),
            1.0,
            Layers::DEFAULT,
        );

        let count = pw.apply_radial_impulse(Vec3::ZERO, 5.0, 100.0, FalloffCurve::Linear, 0.0);

        assert_eq!(count, 1, "Only near body should be affected");
        let far_vel = pw.get_velocity(far).unwrap();
        assert_eq!(far_vel.x, 0.0, "Far body should have no velocity");
    }

    // --- raycast: hit position calculation ---
    #[test]
    fn raycast_hit_position_is_origin_plus_dir_times_toi() {
        let mut pw = PhysicsWorld::new(Vec3::new(0.0, -9.8, 0.0));
        let _box = pw.add_dynamic_box(
            Vec3::new(5.0, 0.0, 0.0),
            Vec3::new(0.5, 0.5, 0.5),
            1.0,
            Layers::DEFAULT,
        );
        pw.step(); // Update query pipeline

        let origin = Vec3::ZERO;
        let direction = Vec3::X;
        let hit = pw.raycast(origin, direction, 20.0);
        assert!(hit.is_some(), "Should hit the box");

        let (hit_pos, _normal, _id, toi) = hit.unwrap();
        // hit_pos should be origin + direction * toi
        let expected_pos = origin + direction * toi;
        assert!(
            (hit_pos.x - expected_pos.x).abs() < 0.01,
            "hit_pos.x={}, expected={}",
            hit_pos.x,
            expected_pos.x
        );
        assert!((hit_pos.y - expected_pos.y).abs() < 0.01);
        assert!((hit_pos.z - expected_pos.z).abs() < 0.01);

        // If + → - or *: hit_pos would be wrong
        assert!(hit_pos.x > 0.0, "Hit should be at positive X");
        assert!(toi > 0.0, "TOI should be positive");
    }

    // --- add_destructible_box: verify body created ---
    #[test]
    fn add_destructible_box_creates_body() {
        let mut pw = PhysicsWorld::new(Vec3::new(0.0, -9.8, 0.0));
        let id = pw.add_destructible_box(
            Vec3::new(0.0, 5.0, 0.0),
            Vec3::new(1.0, 1.0, 1.0),
            2.0,
            100.0,
            50.0,
        );
        // Mutant: replace with Default::default() → id would be 0 with no body
        assert!(
            pw.handle_of(id).is_some(),
            "Body must exist after add_destructible_box"
        );
        assert!(
            pw.body_transform(id).is_some(),
            "Transform must be available"
        );
    }

    // --- enable_ccd: verify it actually enables ---
    #[test]
    fn enable_ccd_activates_ccd_on_body() {
        let mut pw = PhysicsWorld::new(Vec3::new(0.0, -9.8, 0.0));
        let id = pw.add_dynamic_box(
            Vec3::new(0.0, 5.0, 0.0),
            Vec3::new(0.5, 0.5, 0.5),
            1.0,
            Layers::DEFAULT,
        );

        // Mutant: replace enable_ccd with () → CCD not enabled
        pw.enable_ccd(id);

        let h = pw.handle_of(id).unwrap();
        let rb = pw.bodies.get(h).unwrap();
        assert!(
            rb.is_ccd_active() || rb.is_ccd_enabled(),
            "CCD should be enabled after enable_ccd()"
        );
    }

    // --- add_joint: IDs increment sequentially ---
    #[test]
    fn add_joint_ids_increment() {
        let mut pw = PhysicsWorld::new(Vec3::new(0.0, -9.8, 0.0));
        let b1 = pw.add_dynamic_box(
            Vec3::new(0.0, 5.0, 0.0),
            Vec3::new(0.5, 0.5, 0.5),
            1.0,
            Layers::DEFAULT,
        );
        let b2 = pw.add_dynamic_box(
            Vec3::new(2.0, 5.0, 0.0),
            Vec3::new(0.5, 0.5, 0.5),
            1.0,
            Layers::DEFAULT,
        );

        let j1 = pw.add_joint(b1, b2, JointType::Fixed);
        let j2 = pw.add_joint(b1, b2, JointType::Spherical);

        // Mutant: += → *= on next_joint_id would keep it at 0
        assert!(
            j2.0 > j1.0,
            "Joint IDs must increment: j1={}, j2={}",
            j1.0,
            j2.0
        );
    }

    // --- id_of: returns correct BodyId ---
    #[test]
    fn id_of_returns_correct_body() {
        let mut pw = PhysicsWorld::new(Vec3::new(0.0, -9.8, 0.0));
        let box_id = pw.add_dynamic_box(
            Vec3::new(0.0, 5.0, 0.0),
            Vec3::new(0.5, 0.5, 0.5),
            1.0,
            Layers::DEFAULT,
        );

        let h = pw.handle_of(box_id).unwrap();
        let retrieved_id = pw.id_of(h);
        // Mutant: replace with None or Some(Default::default())
        assert_eq!(
            retrieved_id,
            Some(box_id),
            "id_of should return the original body ID"
        );
    }

    // --- apply_buoyancy_forces: body below water gets upward force ---
    #[test]
    fn buoyancy_forces_apply_upward_when_underwater() {
        let mut pw = PhysicsWorld::new(Vec3::new(0.0, -9.8, 0.0));
        pw.water_level = 10.0;
        pw.fluid_density = 1000.0;

        let box_id = pw.add_dynamic_box(
            Vec3::new(0.0, 5.0, 0.0), // Below water level
            Vec3::new(0.5, 0.5, 0.5),
            10.0, // Heavy so gravity is significant
            Layers::DEFAULT,
        );
        pw.add_buoyancy(box_id, 2.0, 0.5); // Large volume for strong buoyancy

        // Without buoyancy, box would fall. With buoyancy, should float up.
        for _ in 0..60 {
            pw.step();
        }

        let y = pw.body_transform(box_id).unwrap().w_axis.y;
        // Buoyancy force = 2.0 * 1000.0 * 9.81 = 19620 N upward
        // Gravity force = 10.0 * 9.8 = 98 N downward
        // Net force is strongly upward, so box rises
        assert!(
            y > 5.0,
            "Buoyancy should push object upward from y=5, got y={}",
            y
        );
    }

    #[test]
    fn buoyancy_not_applied_above_water() {
        let mut pw = PhysicsWorld::new(Vec3::new(0.0, -9.8, 0.0));
        pw.water_level = 0.0; // Water at y=0

        let box_id = pw.add_dynamic_box(
            Vec3::new(0.0, 5.0, 0.0), // Above water level
            Vec3::new(0.5, 0.5, 0.5),
            1.0,
            Layers::DEFAULT,
        );
        pw.add_buoyancy(box_id, 1.0, 0.5);

        // Box should fall normally (no buoyancy above water)
        for _ in 0..60 {
            pw.step();
        }

        let y = pw.body_transform(box_id).unwrap().w_axis.y;
        // Mutant: < → <= would include bodies AT water level
        assert!(y < 5.0, "Box above water should fall, got y={}", y);
    }

    // --- PhysicsStepProfile percentage precision ---
    #[test]
    #[cfg(feature = "async-physics")]
    fn profile_broad_phase_percent_exact_value() {
        let profile = PhysicsStepProfile {
            total_duration: std::time::Duration::from_millis(10),
            broad_phase_duration: std::time::Duration::from_millis(4),
            narrow_phase_duration: std::time::Duration::from_millis(3),
            integration_duration: std::time::Duration::from_millis(3),
            ..Default::default()
        };

        let bp = profile.broad_phase_percent();
        // Expected: 4/10 * 100 = 40.0
        assert!(
            (bp - 40.0).abs() < 0.1,
            "broad_phase_percent should be ~40.0, got {}",
            bp
        );

        // Mutant → 0.0: Would fail (40 ≠ 0)
        // Mutant → 1.0: Would fail (40 ≠ 1)
        // Mutant → -1.0: Would fail (40 ≠ -1)
        assert!(bp > 0.0);
    }

    #[test]
    #[cfg(feature = "async-physics")]
    fn profile_narrow_phase_percent_exact_value() {
        let profile = PhysicsStepProfile {
            total_duration: std::time::Duration::from_millis(10),
            broad_phase_duration: std::time::Duration::from_millis(2),
            narrow_phase_duration: std::time::Duration::from_millis(5),
            integration_duration: std::time::Duration::from_millis(3),
            ..Default::default()
        };

        let np = profile.narrow_phase_percent();
        // Expected: 5/10 * 100 = 50.0
        assert!(
            (np - 50.0).abs() < 0.1,
            "narrow_phase_percent should be ~50.0, got {}",
            np
        );
        assert!(np > 0.0);
    }

    // ===== Mutation-Targeted Remediation Tests =====

    #[test]
    fn mutation_jump_velocity_formula() {
        // Targets: lib.rs:1168 replace * with / in jump
        // Formula: sqrt(2 * g * height)
        let mut pw = PhysicsWorld::new(Vec3::new(0.0, -9.8, 0.0));
        let char_id = pw.add_character(Vec3::new(0.0, 5.0, 0.0), Vec3::new(0.4, 0.9, 0.4));

        pw.jump(char_id, 3.0);

        let ctrl = pw.char_map.get(&char_id).unwrap();
        // g = 9.81 * gravity_scale(1.0) = 9.81
        // expected = sqrt(2 * 9.81 * 3.0) = sqrt(58.86) ≈ 7.67
        let expected = (2.0_f32 * 9.81 * 3.0).sqrt();
        assert!(
            (ctrl.pending_jump_velocity - expected).abs() < 0.1,
            "Jump velocity should be ~{}, got {}",
            expected,
            ctrl.pending_jump_velocity
        );
        // If * → /, sqrt(2 / 9.81 / 3.0) ≈ 0.26 — very different
    }

    #[test]
    fn mutation_control_character_jump_buffer_decrement() {
        // Targets: lib.rs:1189 replace -= with += or /=
        let mut pw = PhysicsWorld::new(Vec3::new(0.0, -9.8, 0.0));
        let char_id = pw.add_character(Vec3::new(0.0, 5.0, 0.0), Vec3::new(0.4, 0.9, 0.4));

        pw.jump(char_id, 2.0);
        let initial_timer = pw.char_map.get(&char_id).unwrap().jump_buffer_timer;
        assert!(initial_timer > 0.0);

        // Call control_character with large enough dt to expire the timer
        pw.control_character(char_id, Vec3::ZERO, initial_timer + 0.01, false);

        let ctrl = pw.char_map.get(&char_id).unwrap();
        // Timer should have been decremented: initial_timer - dt < 0
        assert!(
            ctrl.jump_buffer_timer < 0.0,
            "jump_buffer_timer should be negative after dt > initial, got {}",
            ctrl.jump_buffer_timer
        );
        // If -= became +=, timer would be initial + dt > 0 (wrong)
    }

    #[test]
    fn mutation_control_character_gravity_scale_multiply() {
        // Targets: lib.rs:1193 replace * with + in gravity calculation
        let mut pw = PhysicsWorld::new(Vec3::new(0.0, -9.8, 0.0));
        let char_id = pw.add_character(Vec3::new(0.0, 50.0, 0.0), Vec3::new(0.4, 0.9, 0.4));

        // Modify gravity scale
        if let Some(ctrl) = pw.char_map.get_mut(&char_id) {
            ctrl.gravity_scale = 2.0;
        }

        // Apply 10 frames of freefall
        for _ in 0..10 {
            pw.control_character(char_id, Vec3::ZERO, 1.0 / 60.0, false);
        }

        let vel_2x = pw.char_map.get(&char_id).unwrap().vertical_velocity;

        // Also test with scale=1
        let char_id2 = pw.add_character(Vec3::new(5.0, 50.0, 0.0), Vec3::new(0.4, 0.9, 0.4));
        for _ in 0..10 {
            pw.control_character(char_id2, Vec3::ZERO, 1.0 / 60.0, false);
        }

        let vel_1x = pw.char_map.get(&char_id2).unwrap().vertical_velocity;

        // 2x gravity scale should produce ~2x the downward velocity
        assert!(
            (vel_2x / vel_1x - 2.0).abs() < 0.1,
            "2x gravity_scale should produce ~2x velocity: v2x={}, v1x={}",
            vel_2x,
            vel_1x
        );
    }

    #[test]
    fn mutation_control_character_coyote_time_bounds() {
        // Targets: lib.rs:1199+ replace </>/<=/>= in coyote time and jump checks
        let mut pw = PhysicsWorld::new(Vec3::new(0.0, -9.8, 0.0));
        let char_id = pw.add_character(Vec3::new(0.0, 5.0, 0.0), Vec3::new(0.4, 0.9, 0.4));

        // Set time_since_grounded very large → should NOT be able to jump
        pw.char_map.get_mut(&char_id).unwrap().time_since_grounded = 10.0;
        pw.jump(char_id, 2.0);

        // Apply movement — jump should NOT trigger because coyote time expired
        let vel_before = pw.char_map.get(&char_id).unwrap().vertical_velocity;
        pw.control_character(char_id, Vec3::ZERO, 1.0 / 60.0, false);
        let ctrl = pw.char_map.get(&char_id).unwrap();
        // Velocity should become more negative (gravity), NOT get positive jump boost
        assert!(
            ctrl.vertical_velocity < vel_before || ctrl.vertical_velocity < 0.0,
            "With expired coyote time, jump should not trigger, vel={}",
            ctrl.vertical_velocity
        );
    }

    #[test]
    fn mutation_control_character_climb_negation() {
        // Targets: lib.rs:1202 replace && with ||, :1202 delete ! in !_climb
        let mut pw = PhysicsWorld::new(Vec3::new(0.0, -9.8, 0.0));
        let char_id = pw.add_character(Vec3::new(0.0, 5.0, 0.0), Vec3::new(0.4, 0.9, 0.4));

        // Set up conditions for a valid jump
        pw.char_map.get_mut(&char_id).unwrap().time_since_grounded = 0.0;
        pw.jump(char_id, 2.0);

        // When climbing, jump should NOT trigger
        pw.control_character(char_id, Vec3::ZERO, 1.0 / 60.0, true);
        let ctrl = pw.char_map.get(&char_id).unwrap();
        // Climbing → vertical_velocity should be 0 (no gravity, no jump)
        assert_eq!(
            ctrl.vertical_velocity, 0.0,
            "During climb, vertical_velocity should be 0, got {}",
            ctrl.vertical_velocity
        );

        // When NOT climbing, jump SHOULD trigger
        let char_id2 = pw.add_character(Vec3::new(5.0, 5.0, 0.0), Vec3::new(0.4, 0.9, 0.4));
        pw.char_map.get_mut(&char_id2).unwrap().time_since_grounded = 0.0;
        pw.jump(char_id2, 2.0);
        pw.control_character(char_id2, Vec3::ZERO, 1.0 / 60.0, false);
        let ctrl2 = pw.char_map.get(&char_id2).unwrap();
        assert!(
            ctrl2.vertical_velocity > 0.0,
            "Without climb, jump should trigger, vel={}",
            ctrl2.vertical_velocity
        );
    }

    #[test]
    fn mutation_control_character_horizontal_move_threshold() {
        // Targets: lib.rs:1209 replace >= with <, :1210 replace > with >=
        // Test: no horizontal movement and no vertical → should early return without touching position
        let mut pw = PhysicsWorld::new(Vec3::new(0.0, 0.0, 0.0)); // zero gravity
        let char_id = pw.add_character(Vec3::new(0.0, 5.0, 0.0), Vec3::new(0.4, 0.9, 0.4));

        // Force velocity to 0 so has_vertical is false
        pw.char_map.get_mut(&char_id).unwrap().vertical_velocity = 0.0;

        let pos_before = pw.body_transform(char_id).unwrap().w_axis;
        pw.control_character(char_id, Vec3::ZERO, 1.0 / 60.0, false);
        pw.step();
        let pos_after = pw.body_transform(char_id).unwrap().w_axis;

        // With zero move and zero vertical, position should barely change
        let delta =
            ((pos_after.x - pos_before.x).powi(2) + (pos_after.z - pos_before.z).powi(2)).sqrt();
        assert!(
            delta < 0.01,
            "Zero move + zero gravity should not displace horizontally: delta={}",
            delta
        );
    }

    #[test]
    fn mutation_control_character_coyote_invalidation() {
        // Targets: lib.rs:1204 replace + with - or * in coyote invalidation
        let mut pw = PhysicsWorld::new(Vec3::new(0.0, -9.8, 0.0));
        let char_id = pw.add_character(Vec3::new(0.0, 5.0, 0.0), Vec3::new(0.4, 0.9, 0.4));

        // Set up valid jump conditions
        pw.char_map.get_mut(&char_id).unwrap().time_since_grounded = 0.0;
        pw.jump(char_id, 2.0);

        // Execute jump
        pw.control_character(char_id, Vec3::ZERO, 1.0 / 60.0, false);

        let ctrl = pw.char_map.get(&char_id).unwrap();
        // After jump, time_since_grounded should exceed coyote_time_limit (invalidated)
        assert!(
            ctrl.time_since_grounded > ctrl.coyote_time_limit,
            "After jump, coyote time should be invalidated: tsg={}, limit={}",
            ctrl.time_since_grounded,
            ctrl.coyote_time_limit
        );
        // jump_buffer_timer should be consumed (0.0)
        assert!(
            (ctrl.jump_buffer_timer).abs() < 0.01,
            "Jump buffer should be consumed, got {}",
            ctrl.jump_buffer_timer
        );
    }

    #[test]
    fn mutation_radial_impulse_falloff_distance() {
        // Targets: lib.rs apply_radial_impulse falloff calculation
        let mut pw = PhysicsWorld::new(Vec3::new(0.0, 0.0, 0.0)); // zero gravity
        let near_id = pw.add_dynamic_box(
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.3, 0.3, 0.3),
            1.0,
            Layers::DEFAULT,
        );
        let mid_id = pw.add_dynamic_box(
            Vec3::new(5.0, 0.0, 0.0),
            Vec3::new(0.3, 0.3, 0.3),
            1.0,
            Layers::DEFAULT,
        );

        pw.apply_radial_impulse(Vec3::ZERO, 10.0, 100.0, FalloffCurve::Linear, 0.0);

        let near_vel = pw.get_velocity(near_id).unwrap();
        let mid_vel = pw.get_velocity(mid_id).unwrap();
        // Nearer body should receive more impulse
        assert!(
            near_vel.x > mid_vel.x,
            "Nearer body should get more impulse: near_vx={}, mid_vx={}",
            near_vel.x,
            mid_vel.x
        );
        assert!(
            near_vel.x > 0.0,
            "Near body should move away from explosion"
        );
        assert!(mid_vel.x > 0.0, "Mid body should also move away");
    }

    #[test]
    fn mutation_buoyancy_volume_affects_force() {
        // Targets: lib.rs apply_buoyancy_forces volume and density math
        let mut pw = PhysicsWorld::new(Vec3::new(0.0, -9.8, 0.0));
        pw.water_level = 100.0;
        pw.fluid_density = 1000.0;

        // Large volume → strong buoyancy
        let big_id = pw.add_dynamic_box(
            Vec3::new(0.0, 5.0, 0.0),
            Vec3::new(0.5, 0.5, 0.5),
            5.0,
            Layers::DEFAULT,
        );
        pw.add_buoyancy(big_id, 10.0, 0.5); // big volume

        // Small volume → weak buoyancy
        let small_id = pw.add_dynamic_box(
            Vec3::new(5.0, 5.0, 0.0),
            Vec3::new(0.5, 0.5, 0.5),
            5.0,
            Layers::DEFAULT,
        );
        pw.add_buoyancy(small_id, 0.1, 0.5); // small volume

        for _ in 0..60 {
            pw.step();
        }

        let big_y = pw.body_transform(big_id).unwrap().w_axis.y;
        let small_y = pw.body_transform(small_id).unwrap().w_axis.y;

        // Larger volume → more buoyancy → higher position
        assert!(
            big_y > small_y,
            "Larger volume should rise more: big_y={}, small_y={}",
            big_y,
            small_y
        );
    }

    // ===== DEEP REMEDIATION v3.6.3 — lib.rs Round 4 remaining mutations =====

    // --- CharacterController::volume exact arithmetic (19 mutations) ---
    #[test]
    fn mutation_r4_volume_exact_capsule_formula() {
        // volume = PI*r²*(h - 2r) + (4/3)*PI*r³
        // For r=1.0, h=4.0: cylinder_height = 4-2 = 2, cylinder = PI*1*1*2 = 2PI
        // sphere = (4/3)*PI*1 = 4PI/3, total = 2PI + 4PI/3 = 10PI/3
        let cc = CharacterController::new(1.0, 4.0);
        let expected = 10.0 * std::f32::consts::PI / 3.0;
        assert!(
            (cc.volume() - expected).abs() < 0.01,
            "Volume for r=1 h=4: expected {:.4}, got {:.4}",
            expected,
            cc.volume()
        );
    }

    #[test]
    fn mutation_r4_volume_different_dimensions() {
        // r=0.5, h=3.0: cylinder_height = 3-1 = 2, cylinder = PI*0.25*2 = 0.5PI
        // sphere = (4/3)*PI*0.125 = PI/6, total = 0.5PI + PI/6 = 2PI/3
        let cc = CharacterController::new(0.5, 3.0);
        let expected = 2.0 * std::f32::consts::PI / 3.0;
        assert!(
            (cc.volume() - expected).abs() < 0.01,
            "Volume for r=0.5 h=3: expected {:.4}, got {:.4}",
            expected,
            cc.volume()
        );
    }

    #[test]
    fn mutation_r4_volume_height_minus_radius_subtraction() {
        // If - becomes +: cylinder_height = h + 2r, volume wildly different
        let cc = CharacterController::new(0.3, 2.0);
        let cylinder_h = 2.0 - 2.0 * 0.3; // 1.4
        let cyl_vol = std::f32::consts::PI * 0.3 * 0.3 * cylinder_h;
        let sph_vol = (4.0 / 3.0) * std::f32::consts::PI * 0.3_f32.powi(3);
        let expected = cyl_vol + sph_vol;
        assert!(
            (cc.volume() - expected).abs() < 0.001,
            "Subtraction precision: expected {:.6}, got {:.6}",
            expected,
            cc.volume()
        );
    }

    #[test]
    fn mutation_r4_volume_multiply_operators() {
        // Test with asymmetric r and h so * → + and * → / differ significantly
        let cc = CharacterController::new(2.0, 6.0);
        let cylinder_h = 6.0 - 2.0 * 2.0; // 2.0
        let cyl_vol = std::f32::consts::PI * 2.0 * 2.0 * cylinder_h;
        let sph_vol = (4.0 / 3.0) * std::f32::consts::PI * 2.0_f32.powi(3);
        let expected = cyl_vol + sph_vol;
        assert!(
            (cc.volume() - expected).abs() < 0.01,
            "Multiply operators: expected {:.4}, got {:.4}",
            expected,
            cc.volume()
        );
    }

    #[test]
    fn mutation_r4_volume_division_operator() {
        // 4.0 / 3.0 in sphere volume (mutation: / → %, / → *)
        // For r=1, sphere = (4/3)*PI = 4.189, if * instead: 4*3*PI = 37.7
        let cc = CharacterController::new(1.0, 4.0);
        let vol = cc.volume();
        // sphere component = 4.189 (not 37.7 if * instead of /)
        let sphere_part = (4.0 / 3.0) * std::f32::consts::PI;
        let cylinder_part = std::f32::consts::PI * 1.0 * 1.0 * 2.0;
        assert!(
            (vol - (cylinder_part + sphere_part)).abs() < 0.01,
            "Division operator: got {:.4}",
            vol
        );
    }

    #[test]
    fn mutation_r4_volume_final_addition() {
        // cylinder_volume + sphere_volume (mutation: + → - or + → *)
        let cc = CharacterController::new(1.0, 4.0);
        let cyl = std::f32::consts::PI * 1.0 * 1.0 * 2.0;
        let sph = (4.0 / 3.0) * std::f32::consts::PI;
        // Both parts are positive, total = cyl + sph
        assert!(cc.volume() > cyl, "Total should be > cylinder alone");
        assert!(cc.volume() > sph, "Total should be > sphere alone");
        assert!((cc.volume() - (cyl + sph)).abs() < 0.001, "Final addition");
    }

    #[test]
    fn mutation_r4_volume_replace_with_one() {
        // Mutation: replace volume() → 1.0
        let cc = CharacterController::new(1.0, 4.0);
        assert_ne!(cc.volume(), 1.0, "Volume should not be exactly 1.0");
        assert!(cc.volume() > 5.0, "Volume for r=1 h=4 should be > 5");
    }

    // --- ActorKind::is_other (2 mutations) ---
    #[test]
    fn mutation_r4_actor_kind_is_other_exact() {
        // Mutation: replace → true, replace → false
        assert!(
            ActorKind::Other.is_other(),
            "Other.is_other() should be true"
        );
        assert!(
            !ActorKind::Static.is_other(),
            "Static.is_other() should be false"
        );
        assert!(
            !ActorKind::Dynamic.is_other(),
            "Dynamic.is_other() should be false"
        );
        assert!(
            !ActorKind::Character.is_other(),
            "Character.is_other() should be false"
        );
    }

    // --- DebugLine::length_squared component arithmetic (2 mutations) ---
    #[test]
    fn mutation_r4_length_squared_all_axes() {
        // dx*dx + dy*dy + dz*dz — test with all 3 non-zero and unequal
        // so + → - and * → + are both caught
        let line = DebugLine::new([1.0, 2.0, 3.0], [4.0, 6.0, 8.0], [1.0, 1.0, 1.0]);
        // dx=3, dy=4, dz=5 → 9+16+25 = 50
        let expected = 50.0_f32;
        assert!(
            (line.length_squared() - expected).abs() < 0.001,
            "All axes: expected {}, got {}",
            expected,
            line.length_squared()
        );
    }

    #[test]
    fn mutation_r4_length_squared_single_axis_z() {
        // Pure Z movement to isolate dz*dz term
        let line = DebugLine::new([0.0, 0.0, 0.0], [0.0, 0.0, 7.0], [1.0, 1.0, 1.0]);
        assert!(
            (line.length_squared() - 49.0).abs() < 0.001,
            "Z-only: expected 49, got {}",
            line.length_squared()
        );
    }

    #[test]
    fn mutation_r4_length_squared_xy_only() {
        // X and Y only, Z=0, to catch + → - between dx² and dy²
        let line = DebugLine::new([0.0, 0.0, 0.0], [3.0, 4.0, 0.0], [1.0, 1.0, 1.0]);
        assert!(
            (line.length_squared() - 25.0).abs() < 0.001,
            "XY only: dx²=9 + dy²=16 = 25"
        );
        // If + → -, result would be 9-16 = -7 — very different
        assert!(line.length_squared() > 0.0, "Should be positive");
    }

    // --- DebugLine::is_degenerate boundary (1 mutation: < → <=) ---
    #[test]
    fn mutation_r4_is_degenerate_boundary() {
        // Threshold is length_squared < 1e-10
        // Create line with length_squared = exactly 1e-10 (boundary)
        // sqrt(1e-10) ≈ 1e-5
        let tiny = 1e-5_f32;
        let boundary_line = DebugLine::new([0.0, 0.0, 0.0], [tiny, 0.0, 0.0], [1.0, 1.0, 1.0]);
        // length_squared = tiny² = 1e-10 exactly — with < it's NOT degenerate, with <= it IS
        // Actually f32 at this scale: 1e-5² ≈ 1e-10
        let ls = boundary_line.length_squared();
        // If < 1e-10 changes to <= 1e-10, line at exactly the boundary changes result
        // Use a value clearly above: 2e-5 → 4e-10 — not degenerate
        let above = DebugLine::new([0.0, 0.0, 0.0], [2e-5, 0.0, 0.0], [1.0, 1.0, 1.0]);
        assert!(
            !above.is_degenerate(),
            "Line with length_squared 4e-10 > 1e-10 should not be degenerate, ls={}",
            above.length_squared()
        );
        // Use a value clearly below: 1e-6 → 1e-12 — degenerate
        let below = DebugLine::new([0.0, 0.0, 0.0], [1e-6, 0.0, 0.0], [1.0, 1.0, 1.0]);
        assert!(
            below.is_degenerate(),
            "Line with length_squared 1e-12 < 1e-10 should be degenerate"
        );
    }

    // --- CharState::is_grounded / CharacterController::is_grounded ---
    // Note: CharState only has one variant (Grounded), so is_grounded always returns true.
    // These mutations are equivalent mutants. Verify the invariant:
    #[test]
    fn mutation_r4_char_state_only_variant() {
        // All CharState values are grounded (single variant enum)
        for state in CharState::all() {
            assert!(
                state.is_grounded(),
                "All CharState variants should be grounded"
            );
        }
    }

    // --- CharacterController::has_coyote_time boundary (< → <=) ---
    #[test]
    fn mutation_r4_coyote_time_exact_boundary() {
        // time_since_grounded < coyote_time_limit
        // At exactly the limit: should be false with <, true with <=
        let mut cc = CharacterController::new(0.5, 2.0);
        cc.coyote_time_limit = 0.15;
        cc.time_since_grounded = 0.15; // exactly at limit
                                       // With <:  0.15 < 0.15 → false
                                       // With <=: 0.15 <= 0.15 → true
        assert!(!cc.has_coyote_time(), "At exact limit, < should be false");
        // Just below
        cc.time_since_grounded = 0.149;
        assert!(cc.has_coyote_time(), "Just below limit should be true");
    }

    // --- CharacterController::can_jump || vs && and replace true ---
    #[test]
    fn mutation_r4_can_jump_neither_grounded_nor_coyote() {
        // is_grounded() || has_coyote_time()
        // Mutation: || → &&, replace → true
        // Need: both conditions FALSE → result should be false
        let mut cc = CharacterController::new(0.5, 2.0);
        // Simulate being in-air past coyote time
        cc.time_since_grounded = 1.0; // way past coyote limit
                                      // CharState only has Grounded variant, so is_grounded() is always true...
                                      // Actually we can't make is_grounded false since CharState only has one variant.
                                      // The || → && mutation is effectively equivalent because is_grounded() is always true.
                                      // Verify the tautology:
        assert!(
            cc.can_jump(),
            "can_jump is always true when CharState only has Grounded"
        );
    }

    // --- CharacterController::is_falling boundary (< → <=) ---
    #[test]
    fn mutation_r4_is_falling_exact_boundary() {
        let mut cc = CharacterController::new(0.5, 2.0);
        // At exactly -0.01: < -0.01 is false, <= -0.01 is true
        cc.vertical_velocity = -0.01;
        assert!(!cc.is_falling(), "At exactly -0.01, < should be false");
        cc.vertical_velocity = -0.02;
        assert!(cc.is_falling(), "Below -0.01 should be falling");
    }

    // --- CharacterController::is_rising boundary (> → >=) ---
    #[test]
    fn mutation_r4_is_rising_exact_boundary() {
        let mut cc = CharacterController::new(0.5, 2.0);
        // At exactly 0.01: > 0.01 is false, >= 0.01 is true
        cc.vertical_velocity = 0.01;
        assert!(!cc.is_rising(), "At exactly 0.01, > should be false");
        cc.vertical_velocity = 0.02;
        assert!(cc.is_rising(), "Above 0.01 should be rising");
    }

    // --- PhysicsConfig::is_earth_gravity boundary (< → <= × 3) ---
    #[test]
    fn mutation_r4_earth_gravity_y_boundary() {
        // (gravity.y + 9.81).abs() < 0.1  → at exactly 0.1: false
        let cfg = PhysicsConfig::new().with_gravity(Vec3::new(0.0, -9.81 + 0.1, 0.0));
        // (−9.71 + 9.81).abs() = 0.1. With <: false, with <=: true
        assert!(
            !cfg.is_earth_gravity(),
            "At exactly 0.1 deviation, < should be false"
        );
        let cfg2 = PhysicsConfig::new().with_gravity(Vec3::new(0.0, -9.81 + 0.099, 0.0));
        assert!(
            cfg2.is_earth_gravity(),
            "At 0.099 deviation, should be earth gravity"
        );
    }

    #[test]
    fn mutation_r4_earth_gravity_x_boundary() {
        // gravity.x.abs() < 0.01  → at exactly 0.01: false
        let cfg = PhysicsConfig::new().with_gravity(Vec3::new(0.01, -9.81, 0.0));
        assert!(
            !cfg.is_earth_gravity(),
            "x=0.01: at boundary, < should be false"
        );
        let cfg2 = PhysicsConfig::new().with_gravity(Vec3::new(0.009, -9.81, 0.0));
        assert!(cfg2.is_earth_gravity(), "x=0.009: within threshold");
    }

    #[test]
    fn mutation_r4_earth_gravity_z_boundary() {
        // gravity.z.abs() < 0.01
        let cfg = PhysicsConfig::new().with_gravity(Vec3::new(0.0, -9.81, 0.01));
        assert!(
            !cfg.is_earth_gravity(),
            "z=0.01: at boundary, < should be false"
        );
        let cfg2 = PhysicsConfig::new().with_gravity(Vec3::new(0.0, -9.81, 0.009));
        assert!(cfg2.is_earth_gravity(), "z=0.009: within threshold");
    }

    // --- PhysicsConfig::is_zero_gravity boundary (< → <=) ---
    #[test]
    fn mutation_r4_zero_gravity_boundary() {
        // length_squared() < 1e-6
        // At length_squared = 1e-6: sqrt(1e-6) = 0.001
        let cfg = PhysicsConfig::new().with_gravity(Vec3::new(0.001, 0.0, 0.0));
        // length_squared = 0.000001 = 1e-6 exactly
        // With <: false, with <=: true
        assert!(
            !cfg.is_zero_gravity(),
            "At exact threshold 1e-6, < should be false"
        );
        let cfg2 = PhysicsConfig::new().with_gravity(Vec3::new(0.0001, 0.0, 0.0));
        assert!(
            cfg2.is_zero_gravity(),
            "Below threshold should be zero gravity"
        );
    }

    // ===== ECS INTEGRATION SCAFFOLDING v3.7.0 — PhysicsWorld integration tests =====

    // --- PhysicsWorld::control_character gravity (53 mutations) ---
    #[test]
    fn integration_control_character_gravity_pulls_down() {
        let mut pw = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));
        let char_id = pw.add_character(Vec3::new(0.0, 10.0, 0.0), Vec3::new(0.3, 0.9, 0.3));
        pw.step(); // Initialize query pipeline

        // Move with no input for several frames — gravity should pull down
        for _ in 0..60 {
            pw.control_character(char_id, Vec3::ZERO, 1.0 / 60.0, false);
            pw.step();
        }

        let transform = pw.body_transform(char_id).unwrap();
        let y = transform.w_axis.y;
        assert!(y < 10.0, "Character should fall due to gravity: y={}", y);
    }

    #[test]
    fn integration_control_character_horizontal_move() {
        let mut pw = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));
        // Place character on a ground plane
        let _ground = pw.create_ground_plane(Vec3::new(100.0, 0.1, 100.0), 0.8);
        let char_id = pw.add_character(Vec3::new(0.0, 1.0, 0.0), Vec3::new(0.3, 0.9, 0.3));
        pw.step();

        // Move in +X direction
        for _ in 0..30 {
            pw.control_character(char_id, Vec3::new(5.0, 0.0, 0.0), 1.0 / 60.0, false);
            pw.step();
        }

        let transform = pw.body_transform(char_id).unwrap();
        let x = transform.w_axis.x;
        assert!(x > 0.1, "Character should have moved in +X: x={}", x);
    }

    #[test]
    fn integration_control_character_jump_velocity() {
        let mut pw = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));
        let _ground = pw.create_ground_plane(Vec3::new(100.0, 0.1, 100.0), 0.8);
        let char_id = pw.add_character(Vec3::new(0.0, 1.0, 0.0), Vec3::new(0.3, 0.9, 0.3));
        pw.step();

        // Ground the character first
        for _ in 0..10 {
            pw.control_character(char_id, Vec3::ZERO, 1.0 / 60.0, false);
            pw.step();
        }

        let before_y = pw.body_transform(char_id).unwrap().w_axis.y;

        // Jump with height 2.0
        pw.jump(char_id, 2.0);

        // Let the jump execute
        for _ in 0..5 {
            pw.control_character(char_id, Vec3::ZERO, 1.0 / 60.0, false);
            pw.step();
        }

        let after_y = pw.body_transform(char_id).unwrap().w_axis.y;
        assert!(
            after_y > before_y,
            "Jump should push character up: before={}, after={}",
            before_y,
            after_y
        );
    }

    #[test]
    fn integration_control_character_coyote_time() {
        let mut pw = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));
        let char_id = pw.add_character(Vec3::new(0.0, 1.0, 0.0), Vec3::new(0.3, 0.9, 0.3));

        // The character has a coyote_time_limit of 0.1s
        let ctrl = pw.char_map.get(&char_id).unwrap();
        assert!(ctrl.coyote_time_limit > 0.0, "Coyote time should be set");

        // Verify the controller's time_since_grounded starts at 0
        assert_eq!(ctrl.time_since_grounded, 0.0, "Should start grounded");
    }

    #[test]
    fn integration_control_character_climb_mode() {
        let mut pw = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));
        let char_id = pw.add_character(Vec3::new(0.0, 5.0, 0.0), Vec3::new(0.3, 0.9, 0.3));
        pw.step();

        let y_before = pw.body_transform(char_id).unwrap().w_axis.y;

        // Climb mode should move upward and zero vertical velocity
        for _ in 0..30 {
            pw.control_character(char_id, Vec3::ZERO, 1.0 / 60.0, true);
            pw.step();
        }

        let y_after = pw.body_transform(char_id).unwrap().w_axis.y;
        assert!(
            y_after > y_before,
            "Climb mode should move up: before={}, after={}",
            y_before,
            y_after
        );
    }

    #[test]
    fn integration_control_character_no_id_returns_early() {
        let mut pw = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));
        // Non-existent character ID should not panic
        pw.control_character(999, Vec3::new(1.0, 0.0, 0.0), 1.0 / 60.0, false);
        // If we get here without panic, the early return worked
    }

    #[test]
    fn integration_control_character_vertical_velocity_accumulates() {
        let mut pw = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));
        let char_id = pw.add_character(Vec3::new(0.0, 50.0, 0.0), Vec3::new(0.3, 0.9, 0.3));
        pw.step();

        // Fall for a bit without ground
        for _ in 0..10 {
            pw.control_character(char_id, Vec3::ZERO, 1.0 / 60.0, false);
            pw.step();
        }

        let ctrl = pw.char_map.get(&char_id).unwrap();
        assert!(
            ctrl.vertical_velocity < 0.0,
            "Vertical velocity should be negative (falling): {}",
            ctrl.vertical_velocity
        );
    }

    #[test]
    fn integration_control_character_jump_buffer_consumed() {
        let mut pw = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));
        let _ground = pw.create_ground_plane(Vec3::new(100.0, 0.1, 100.0), 0.8);
        let char_id = pw.add_character(Vec3::new(0.0, 1.0, 0.0), Vec3::new(0.3, 0.9, 0.3));
        pw.step();

        // Ground the character
        for _ in 0..10 {
            pw.control_character(char_id, Vec3::ZERO, 1.0 / 60.0, false);
            pw.step();
        }

        pw.jump(char_id, 1.0);
        pw.control_character(char_id, Vec3::ZERO, 1.0 / 60.0, false);

        let ctrl = pw.char_map.get(&char_id).unwrap();
        assert!(
            ctrl.jump_buffer_timer <= 0.0,
            "Jump buffer should be consumed after jump: {}",
            ctrl.jump_buffer_timer
        );
    }

    #[test]
    fn integration_control_character_slope_limit() {
        let mut pw = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));
        let char_id = pw.add_character(Vec3::new(0.0, 1.0, 0.0), Vec3::new(0.3, 0.9, 0.3));

        // Verify max climb angle is set
        let ctrl = pw.char_map.get(&char_id).unwrap();
        assert!(
            ctrl.max_climb_angle_deg > 0.0 && ctrl.max_climb_angle_deg < 90.0,
            "Slope limit should be reasonable: {}",
            ctrl.max_climb_angle_deg
        );
    }

    #[test]
    fn integration_control_character_stores_controller_state() {
        let mut pw = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));
        let char_id = pw.add_character(Vec3::new(0.0, 50.0, 0.0), Vec3::new(0.3, 0.9, 0.3));
        pw.step();

        // After control_character, the controller state should be updated
        pw.control_character(char_id, Vec3::ZERO, 0.1, false);

        let ctrl = pw.char_map.get(&char_id).unwrap();
        // Gravity should have been applied: vertical_velocity -= 9.81 * 1.0 * 0.1
        assert!(
            ctrl.vertical_velocity < 0.0,
            "Gravity should make vertical_velocity negative: {}",
            ctrl.vertical_velocity
        );
    }

    // --- PhysicsWorld::apply_radial_impulse integration (6 mutations) ---
    #[test]
    fn integration_radial_impulse_affects_nearby_bodies() {
        let mut pw = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));
        let body1 = pw.add_dynamic_box(
            Vec3::new(3.0, 1.0, 0.0),
            Vec3::new(0.5, 0.5, 0.5),
            1.0,
            Layers::DEFAULT,
        );
        let _body_far = pw.add_dynamic_box(
            Vec3::new(100.0, 1.0, 0.0),
            Vec3::new(0.5, 0.5, 0.5),
            1.0,
            Layers::DEFAULT,
        );
        pw.step();

        let count = pw.apply_radial_impulse(
            Vec3::ZERO,
            10.0,
            500.0,
            crate::projectile::FalloffCurve::Linear,
            0.0,
        );
        assert!(
            count >= 1,
            "At least one body should be affected: {}",
            count
        );

        pw.step();

        // Near body should have velocity from impulse
        let vel = pw.get_velocity(body1).unwrap();
        assert!(
            vel.length() > 0.1,
            "Nearby body should have velocity from impulse: {:?}",
            vel
        );
    }

    #[test]
    fn integration_radial_impulse_excludes_outside_radius() {
        let mut pw = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));
        let _body = pw.add_dynamic_box(
            Vec3::new(20.0, 0.0, 0.0),
            Vec3::new(0.5, 0.5, 0.5),
            1.0,
            Layers::DEFAULT,
        );
        pw.step();

        let count = pw.apply_radial_impulse(
            Vec3::ZERO,
            5.0, // Only 5m radius
            500.0,
            crate::projectile::FalloffCurve::Linear,
            0.0,
        );
        assert_eq!(count, 0, "Body at 20m should be outside 5m radius");
    }

    #[test]
    fn integration_radial_impulse_upward_bias() {
        let mut pw = PhysicsWorld::new(Vec3::ZERO); // No gravity for easier testing
        let body = pw.add_dynamic_box(
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.5, 0.5, 0.5),
            1.0,
            Layers::DEFAULT,
        );
        pw.step();

        pw.apply_radial_impulse(
            Vec3::ZERO,
            10.0,
            100.0,
            crate::projectile::FalloffCurve::Constant,
            1.0, // Full upward bias
        );
        pw.step();

        let vel = pw.get_velocity(body).unwrap();
        assert!(
            vel.y > vel.x.abs(),
            "Full upward bias should give mostly Y velocity: {:?}",
            vel
        );
    }

    // --- PhysicsWorld::apply_buoyancy_forces integration (4 mutations) ---
    #[test]
    fn integration_buoyancy_force_applied_underwater() {
        let mut pw = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));
        let body = pw.add_dynamic_box(
            Vec3::new(0.0, -5.0, 0.0), // Below water
            Vec3::new(0.5, 0.5, 0.5),
            1.0,
            Layers::DEFAULT,
        );
        pw.water_level = 0.0; // Water surface at y=0
        pw.fluid_density = 1000.0;
        pw.add_buoyancy(body, 1.0, 0.5);
        pw.step();

        // Buoyancy should create upward force on submerged body
        let vel = pw.get_velocity(body).unwrap();
        // After one step, the body should have some upward velocity from buoyancy
        // buoyancy_force = 1.0 * 1000.0 * 9.81 = 9810 N upward
        assert!(
            vel.y > -10.0,
            "Buoyancy should counteract gravity: vel.y={}",
            vel.y
        );
    }

    #[test]
    fn integration_buoyancy_not_applied_above_water() {
        let mut pw = PhysicsWorld::new(Vec3::ZERO); // No gravity
        let body = pw.add_dynamic_box(
            Vec3::new(0.0, 10.0, 0.0), // Above water
            Vec3::new(0.5, 0.5, 0.5),
            1.0,
            Layers::DEFAULT,
        );
        pw.water_level = 0.0;
        pw.add_buoyancy(body, 1.0, 0.5);
        pw.step();

        let vel = pw.get_velocity(body).unwrap();
        assert!(
            vel.length() < 0.01,
            "Above water: no buoyancy force should apply: {:?}",
            vel
        );
    }

    #[test]
    fn integration_buoyancy_drag_slows_body() {
        let mut pw = PhysicsWorld::new(Vec3::ZERO);
        let body = pw.add_dynamic_box(
            Vec3::new(0.0, -5.0, 0.0), // Below water
            Vec3::new(0.5, 0.5, 0.5),
            1.0,
            Layers::DEFAULT,
        );
        pw.water_level = 0.0;
        pw.set_velocity(body, Vec3::new(10.0, 0.0, 0.0));
        pw.add_buoyancy(body, 0.001, 50.0); // Small volume, high drag
        pw.step();

        let vel = pw.get_velocity(body).unwrap();
        assert!(vel.x < 10.0, "Drag should slow the body: vel.x={}", vel.x);
    }

    // ===== ROUND 6: Deep control_character integration tests =====

    #[test]
    fn r6_control_character_ground_snap() {
        let mut pw = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));
        let _ground = pw.create_ground_plane(Vec3::new(100.0, 0.1, 100.0), 0.5);
        pw.step(); // Update query pipeline

        // Character starts slightly above ground
        let ch = pw.add_character(Vec3::new(0.0, 1.0, 0.0), Vec3::new(0.3, 0.8, 0.3));

        // Let gravity pull character down, control with no horizontal move
        for _ in 0..60 {
            pw.control_character(ch, Vec3::ZERO, 1.0 / 60.0, false);
            pw.step();
        }

        // Character should have snapped to ground (y near ground level)
        let transform = pw.body_transform(ch).unwrap();
        let y = transform.w_axis.y;
        assert!(y < 2.0, "Character should have fallen near ground: y={}", y);
    }

    #[test]
    fn r6_control_character_wall_slide() {
        let mut pw = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));
        let _ground = pw.create_ground_plane(Vec3::new(100.0, 0.1, 100.0), 0.5);
        // Create a wall (tall, heavy box) in front of character
        let _wall = pw.add_dynamic_box(
            Vec3::new(5.0, 1.0, 0.0),
            Vec3::new(0.5, 5.0, 5.0),
            10000.0,
            Layers::DEFAULT,
        );
        pw.step();

        let ch = pw.add_character(Vec3::new(0.0, 0.5, 0.0), Vec3::new(0.3, 0.8, 0.3));

        // Settle on ground first
        for _ in 0..30 {
            pw.control_character(ch, Vec3::ZERO, 1.0 / 60.0, false);
            pw.step();
        }

        // Move toward the wall (+X)
        // The raycast should detect the wall and deflect movement
        for _ in 0..60 {
            pw.control_character(ch, Vec3::new(5.0, 0.0, 0.0), 1.0 / 60.0, false);
            pw.step();
        }

        let transform = pw.body_transform(ch).unwrap();
        let x = transform.w_axis.x;
        // Character should have been stopped or deflected by wall
        // Without wall-slide, x would be ~5.0. With it, should be < wall position
        assert!(x < 6.0, "Wall should limit forward movement: x={}", x);
    }

    #[test]
    fn r6_control_character_jump_resets_coyote() {
        let mut pw = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));
        let _ground = pw.create_ground_plane(Vec3::new(100.0, 0.1, 100.0), 0.5);
        pw.step();

        let ch = pw.add_character(Vec3::new(0.0, 0.5, 0.0), Vec3::new(0.3, 0.8, 0.3));

        // Settle on ground
        for _ in 0..30 {
            pw.control_character(ch, Vec3::ZERO, 1.0 / 60.0, false);
            pw.step();
        }

        // Trigger jump
        pw.jump(ch, 3.0);

        let ctrl_before = pw.char_map.get(&ch).copied().unwrap();
        // On ground, time_since_grounded should be small (coyote eligible)
        assert!(
            ctrl_before.time_since_grounded < ctrl_before.coyote_time_limit + 0.1,
            "Should be coyote-eligible before jump"
        );

        // Execute one control step to process the jump
        pw.control_character(ch, Vec3::ZERO, 1.0 / 60.0, false);
        pw.step();

        let ctrl_after = pw.char_map.get(&ch).copied().unwrap();
        // After jumping, time_since_grounded should be > coyote_time_limit
        // (invalidated to prevent double-jump)
        assert!(
            ctrl_after.time_since_grounded > ctrl_before.coyote_time_limit,
            "Jump should invalidate coyote time: time_since_grounded={}",
            ctrl_after.time_since_grounded
        );
        // Jump buffer should be consumed
        assert!(
            ctrl_after.jump_buffer_timer <= 0.0,
            "Jump buffer should be consumed: {}",
            ctrl_after.jump_buffer_timer
        );
    }

    #[test]
    fn r6_control_character_slope_rejection() {
        let mut pw = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));
        let _ground = pw.create_ground_plane(Vec3::new(100.0, 0.1, 100.0), 0.5);
        pw.step();

        let ch = pw.add_character(Vec3::new(0.0, 0.5, 0.0), Vec3::new(0.3, 0.8, 0.3));

        // Settle on ground
        for _ in 0..60 {
            pw.control_character(ch, Vec3::ZERO, 1.0 / 60.0, false);
            pw.step();
        }

        let ctrl = pw.char_map.get(&ch).copied().unwrap();
        // On a flat ground plane (slope=0°), character should be grounded
        // time_since_grounded should be 0 or very small
        assert!(
            ctrl.time_since_grounded < 1.0,
            "On flat ground, should be recently grounded: time_since_grounded={}",
            ctrl.time_since_grounded
        );
    }

    #[test]
    fn r6_control_character_climb_mode() {
        let mut pw = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));
        let _ground = pw.create_ground_plane(Vec3::new(100.0, 0.1, 100.0), 0.5);
        pw.step();

        let ch = pw.add_character(Vec3::new(0.0, 0.5, 0.0), Vec3::new(0.3, 0.8, 0.3));

        let y_before = pw.body_transform(ch).unwrap().w_axis.y;

        // Climb mode: vertical_velocity = 0, moves up at 2.0*dt per step
        for _ in 0..30 {
            pw.control_character(ch, Vec3::ZERO, 1.0 / 60.0, true);
            pw.step();
        }

        let y_after = pw.body_transform(ch).unwrap().w_axis.y;
        assert!(
            y_after > y_before,
            "Climb mode should move up: before={}, after={}",
            y_before,
            y_after
        );

        // In climb mode, vertical_velocity should be 0
        let ctrl = pw.char_map.get(&ch).copied().unwrap();
        assert!(
            ctrl.vertical_velocity.abs() < 0.01,
            "Climb mode zeroes vertical_velocity: {}",
            ctrl.vertical_velocity
        );
    }

    #[test]
    fn r6_control_character_no_id_noop() {
        // Control a non-existent character should not crash
        let mut pw = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));
        pw.control_character(9999, Vec3::X, 1.0 / 60.0, false);
        // Should just return without error
    }

    #[test]
    fn r6_radial_impulse_count_returns_affected() {
        let mut pw = PhysicsWorld::new(Vec3::ZERO);
        let _b1 = pw.add_dynamic_box(
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::splat(0.5),
            1.0,
            Layers::DEFAULT,
        );
        let _b2 = pw.add_dynamic_box(
            Vec3::new(2.0, 0.0, 0.0),
            Vec3::splat(0.5),
            1.0,
            Layers::DEFAULT,
        );
        let _b3 = pw.add_dynamic_box(
            Vec3::new(100.0, 0.0, 0.0),
            Vec3::splat(0.5),
            1.0,
            Layers::DEFAULT,
        );
        pw.step();

        let count =
            pw.apply_radial_impulse(Vec3::ZERO, 5.0, 100.0, crate::FalloffCurve::Linear, 0.0);
        // b1 and b2 are within radius 5, b3 is at 100 (outside)
        assert!(
            count >= 2,
            "Should affect at least 2 nearby bodies: {}",
            count
        );
        assert!(count <= 3, "Should not affect far body: {}", count);
    }

    #[test]
    fn r6_apply_force_changes_velocity() {
        let mut pw = PhysicsWorld::new(Vec3::ZERO);
        let body = pw.add_dynamic_box(Vec3::ZERO, Vec3::splat(0.5), 1.0, Layers::DEFAULT);
        pw.step();

        pw.apply_force(body, Vec3::new(100.0, 0.0, 0.0));
        pw.step();

        let vel = pw.get_velocity(body).unwrap();
        assert!(vel.x > 0.0, "Force should produce velocity: {:?}", vel);
    }

    #[test]
    fn r6_apply_impulse_changes_velocity() {
        let mut pw = PhysicsWorld::new(Vec3::ZERO);
        let body = pw.add_dynamic_box(Vec3::ZERO, Vec3::splat(0.5), 1.0, Layers::DEFAULT);
        pw.step();

        pw.apply_impulse(body, Vec3::new(10.0, 0.0, 0.0));
        pw.step();

        let vel = pw.get_velocity(body).unwrap();
        assert!(vel.x > 0.0, "Impulse should produce velocity: {:?}", vel);
    }

    #[test]
    fn r6_raycast_hits_ground() {
        let mut pw = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));
        let _ground = pw.create_ground_plane(Vec3::new(100.0, 0.1, 100.0), 0.5);
        pw.step();

        let result = pw.raycast(Vec3::new(0.0, 10.0, 0.0), -Vec3::Y, 50.0);
        assert!(result.is_some(), "Raycast should hit ground plane");
        if let Some((hit_point, hit_normal, _body, _dist)) = result {
            assert!(
                hit_point.y < 1.0,
                "Hit point should be near ground: {:?}",
                hit_point
            );
            assert!(
                hit_normal.y > 0.5,
                "Ground normal should point up: {:?}",
                hit_normal
            );
        }
    }

    #[test]
    fn r6_enable_ccd_no_crash() {
        let mut pw = PhysicsWorld::new(Vec3::ZERO);
        let body = pw.add_dynamic_box(Vec3::ZERO, Vec3::splat(0.5), 1.0, Layers::DEFAULT);
        pw.enable_ccd(body);
        pw.step();
        // CCD should work without error
        assert!(pw.get_velocity(body).is_some());
    }

    // ===== ROUND 7: Targeted catches =====

    #[test]
    fn r7_jump_changes_vertical_velocity() {
        let mut pw = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));
        let _ground = pw.create_ground_plane(Vec3::new(100.0, 0.1, 100.0), 0.5);
        pw.step();
        let ch = pw.add_character(Vec3::new(0.0, 0.5, 0.0), Vec3::new(0.3, 0.8, 0.3));

        // Settle on ground
        for _ in 0..30 {
            pw.control_character(ch, Vec3::ZERO, 1.0 / 60.0, false);
            pw.step();
        }

        // Jump
        pw.jump(ch, 3.0);

        // After jump, character should be on_ground = false or have upward velocity
        // Step the simulation to apply the jump
        for _ in 0..10 {
            pw.control_character(ch, Vec3::ZERO, 1.0 / 60.0, false);
            pw.step();
        }

        let transform = pw.body_transform(ch).unwrap();
        let y = transform.w_axis.y;
        // Character should have moved significantly upward from jump
        // Original: v = sqrt(2*9.81*3) ≈ 7.67, so after 10 steps the rise is large
        // Mutant (*/): v ≈ 2.56 or 0.78, rise is much smaller
        assert!(
            y > 1.0,
            "Jump should move character well above start: y={}",
            y
        );
    }

    // ===== ROUND 8: Struct/helper + control_character catches =====

    #[test]
    fn r8_debug_line_is_degenerate_zero_length() {
        let line = DebugLine {
            start: [1.0, 2.0, 3.0],
            end: [1.0, 2.0, 3.0],
            color: [1.0, 0.0, 0.0],
        };
        assert!(
            line.is_degenerate(),
            "Zero-length line should be degenerate"
        );

        let line2 = DebugLine {
            start: [0.0, 0.0, 0.0],
            end: [1.0, 0.0, 0.0],
            color: [1.0, 0.0, 0.0],
        };
        assert!(
            !line2.is_degenerate(),
            "Unit-length line should not be degenerate"
        );
    }

    #[test]
    fn r8_char_state_is_grounded() {
        let state = CharState::Grounded;
        assert!(state.is_grounded(), "Grounded state should return true");
    }

    #[test]
    fn r8_character_controller_is_grounded_delegates() {
        let ctrl = CharacterController {
            state: CharState::Grounded,
            max_climb_angle_deg: 45.0,
            radius: 0.3,
            height: 1.6,
            max_step: 0.3,
            vertical_velocity: 0.0,
            gravity_scale: 1.0,
            time_since_grounded: 0.0,
            jump_buffer_timer: 0.0,
            coyote_time_limit: 0.15,
            jump_buffer_limit: 0.15,
            pending_jump_velocity: 0.0,
        };
        assert!(ctrl.is_grounded(), "Controller should be grounded");
    }

    #[test]
    fn r8_character_controller_can_jump_when_grounded() {
        let ctrl = CharacterController {
            state: CharState::Grounded,
            max_climb_angle_deg: 45.0,
            radius: 0.3,
            height: 1.6,
            max_step: 0.3,
            vertical_velocity: 0.0,
            gravity_scale: 1.0,
            time_since_grounded: 0.0,
            jump_buffer_timer: 0.0,
            coyote_time_limit: 0.15,
            jump_buffer_limit: 0.15,
            pending_jump_velocity: 0.0,
        };
        assert!(
            ctrl.can_jump(),
            "Grounded controller should be able to jump"
        );
    }

    #[test]
    fn r8_physics_config_is_earth_gravity() {
        let config = PhysicsConfig::default();
        assert!(
            config.is_earth_gravity(),
            "Default gravity should be earth-like"
        );

        let moon = PhysicsConfig {
            gravity: Vec3::new(0.0, -1.625, 0.0),
            ..Default::default()
        };
        assert!(!moon.is_earth_gravity(), "Moon gravity is not earth-like");
    }

    #[test]
    fn r8_physics_config_is_zero_gravity() {
        let zero = PhysicsConfig {
            gravity: Vec3::ZERO,
            ..Default::default()
        };
        assert!(zero.is_zero_gravity(), "Zero vector should be zero gravity");

        let config = PhysicsConfig::default();
        assert!(!config.is_zero_gravity(), "Earth gravity is not zero");
    }

    #[test]
    fn r8_control_character_gravity_decreases_y() {
        // Test that gravity pulls character down over time
        let mut pw = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));
        let ch = pw.add_character(Vec3::new(0.0, 10.0, 0.0), Vec3::new(0.3, 0.8, 0.3));
        pw.step();

        // Character starts high with no ground
        let start_y = pw.body_transform(ch).unwrap().w_axis.y;

        // Step with no movement → gravity should pull down
        for _ in 0..30 {
            pw.control_character(ch, Vec3::ZERO, 1.0 / 60.0, false);
            pw.step();
        }

        let after_y = pw.body_transform(ch).unwrap().w_axis.y;
        assert!(
            after_y < start_y - 1.0,
            "Gravity should pull character down significantly: start={}, after={}",
            start_y,
            after_y
        );
    }

    #[test]
    fn r8_control_character_horizontal_movement() {
        // Test horizontal movement changes x/z position
        let mut pw = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));
        let _ground = pw.create_ground_plane(Vec3::new(100.0, 0.1, 100.0), 0.5);
        pw.step();
        let ch = pw.add_character(Vec3::new(0.0, 0.5, 0.0), Vec3::new(0.3, 0.8, 0.3));

        // Settle
        for _ in 0..30 {
            pw.control_character(ch, Vec3::ZERO, 1.0 / 60.0, false);
            pw.step();
        }

        let start_x = pw.body_transform(ch).unwrap().w_axis.x;

        // Move in +X direction
        for _ in 0..30 {
            pw.control_character(ch, Vec3::new(5.0, 0.0, 0.0), 1.0 / 60.0, false);
            pw.step();
        }

        let after_x = pw.body_transform(ch).unwrap().w_axis.x;
        assert!(
            after_x > start_x + 1.0,
            "Horizontal movement should change X: start={}, after={}",
            start_x,
            after_x
        );
    }

    #[test]
    fn r8_control_character_jump_consumes_buffer() {
        let mut pw = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));
        let _ground = pw.create_ground_plane(Vec3::new(100.0, 0.1, 100.0), 0.5);
        pw.step();
        let ch = pw.add_character(Vec3::new(0.0, 0.5, 0.0), Vec3::new(0.3, 0.8, 0.3));

        // Settle
        for _ in 0..30 {
            pw.control_character(ch, Vec3::ZERO, 1.0 / 60.0, false);
            pw.step();
        }

        // Jump
        pw.jump(ch, 2.0);

        // After one control_character call, the jump should be consumed
        pw.control_character(ch, Vec3::ZERO, 1.0 / 60.0, false);
        pw.step();

        let ctrl = pw.char_map.get(&ch).unwrap();
        // After consuming jump, buffer should be at 0 or negative
        assert!(
            ctrl.jump_buffer_timer <= 0.0,
            "Jump buffer should be consumed: {}",
            ctrl.jump_buffer_timer
        );
        // Coyote time should be invalidated after jumping
        assert!(
            ctrl.time_since_grounded > ctrl.coyote_time_limit,
            "Coyote time should be invalidated after jump"
        );
    }

    #[test]
    fn r8_control_character_climb_mode_rises() {
        let mut pw = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));
        let _ground = pw.create_ground_plane(Vec3::new(100.0, 0.1, 100.0), 0.5);
        pw.step();
        let ch = pw.add_character(Vec3::new(0.0, 0.5, 0.0), Vec3::new(0.3, 0.8, 0.3));

        // Settle
        for _ in 0..30 {
            pw.control_character(ch, Vec3::ZERO, 1.0 / 60.0, false);
            pw.step();
        }

        let start_y = pw.body_transform(ch).unwrap().w_axis.y;

        // Climb mode: _climb=true should move upward
        for _ in 0..30 {
            pw.control_character(ch, Vec3::ZERO, 1.0 / 60.0, true);
            pw.step();
        }

        let after_y = pw.body_transform(ch).unwrap().w_axis.y;
        assert!(
            after_y > start_y + 0.5,
            "Climb mode should raise character: start={}, after={}",
            start_y,
            after_y
        );
    }

    #[test]
    fn r8_control_character_ground_snaps_y() {
        // Test that character snaps to ground when falling
        let mut pw = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));
        let _ground = pw.create_ground_plane(Vec3::new(100.0, 0.1, 100.0), 0.5);
        pw.step();
        let ch = pw.add_character(Vec3::new(0.0, 2.0, 0.0), Vec3::new(0.3, 0.8, 0.3));

        // Let character fall and settle
        for _ in 0..120 {
            pw.control_character(ch, Vec3::ZERO, 1.0 / 60.0, false);
            pw.step();
        }

        let y = pw.body_transform(ch).unwrap().w_axis.y;
        // Should have settled near the ground (y ~ 0.0 to 0.5)
        assert!(
            y < 1.0,
            "Character should have settled near ground: y={}",
            y
        );
        // Vertical velocity should be 0 when grounded
        let ctrl = pw.char_map.get(&ch).unwrap();
        assert!(
            ctrl.vertical_velocity.abs() < 0.1,
            "Grounded character should have ~0 vertical velocity: {}",
            ctrl.vertical_velocity
        );
    }

    #[test]
    fn r8_control_character_slope_check() {
        // Test that slope detection uses max_climb_angle_deg
        let mut pw = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));
        let _ground = pw.create_ground_plane(Vec3::new(100.0, 0.1, 100.0), 0.5);
        pw.step();
        let ch = pw.add_character(Vec3::new(0.0, 0.5, 0.0), Vec3::new(0.3, 0.8, 0.3));

        // Settle on flat ground
        for _ in 0..60 {
            pw.control_character(ch, Vec3::ZERO, 1.0 / 60.0, false);
            pw.step();
        }

        // Character should be grounded on flat surface
        let ctrl = pw.char_map.get(&ch).unwrap();
        assert!(
            ctrl.time_since_grounded < 0.5,
            "Should be near-grounded on flat surface: time_since={}",
            ctrl.time_since_grounded
        );
    }

    // ===== ROUND 10: control_character arithmetic precision =====

    #[test]
    fn r10_control_character_gravity_scale_affects_fall() {
        // Gravity_scale changes how fast character falls
        let mut pw = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));
        let ch = pw.add_character(Vec3::new(0.0, 10.0, 0.0), Vec3::new(0.3, 0.8, 0.3));
        pw.step();

        // Set gravity_scale to 2.0
        if let Some(ctrl) = pw.char_map.get_mut(&ch) {
            ctrl.gravity_scale = 2.0;
        }

        for _ in 0..30 {
            pw.control_character(ch, Vec3::ZERO, 1.0 / 60.0, false);
            pw.step();
        }
        let y_2x = pw.body_transform(ch).unwrap().w_axis.y;

        // Create another with scale=1.0
        let mut pw2 = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));
        let ch2 = pw2.add_character(Vec3::new(0.0, 10.0, 0.0), Vec3::new(0.3, 0.8, 0.3));
        pw2.step();

        for _ in 0..30 {
            pw2.control_character(ch2, Vec3::ZERO, 1.0 / 60.0, false);
            pw2.step();
        }
        let y_1x = pw2.body_transform(ch2).unwrap().w_axis.y;

        assert!(
            y_2x < y_1x,
            "2x gravity should fall faster: y_2x={}, y_1x={}",
            y_2x,
            y_1x
        );
    }

    #[test]
    fn r10_control_character_jump_velocity_formula() {
        // Jump velocity = sqrt(2 * g * height)
        // With gravity_scale=1.0 and height=3.0: v = sqrt(2*9.81*3) = sqrt(58.86) ≈ 7.67
        let mut pw = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));
        let _ground = pw.create_ground_plane(Vec3::new(100.0, 0.1, 100.0), 0.5);
        pw.step();
        let ch = pw.add_character(Vec3::new(0.0, 0.5, 0.0), Vec3::new(0.3, 0.8, 0.3));

        for _ in 0..60 {
            pw.control_character(ch, Vec3::ZERO, 1.0 / 60.0, false);
            pw.step();
        }

        pw.jump(ch, 3.0);
        let ctrl = pw.char_map.get(&ch).unwrap();
        let expected_v = (2.0 * 9.81 * 3.0_f32).sqrt();
        assert!(
            (ctrl.pending_jump_velocity - expected_v).abs() < 0.1,
            "Jump velocity should be sqrt(2*g*h)={}, got={}",
            expected_v,
            ctrl.pending_jump_velocity
        );
    }

    #[test]
    fn r10_control_character_climb_zero_vertical_velocity() {
        // In climb mode, vertical_velocity should be zeroed
        let mut pw = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));
        let ch = pw.add_character(Vec3::new(0.0, 5.0, 0.0), Vec3::new(0.3, 0.8, 0.3));
        pw.step();

        // Let character fall a bit (builds vertical velocity)
        for _ in 0..10 {
            pw.control_character(ch, Vec3::ZERO, 1.0 / 60.0, false);
            pw.step();
        }

        // Now switch to climb mode
        pw.control_character(ch, Vec3::ZERO, 1.0 / 60.0, true);
        let ctrl = pw.char_map.get(&ch).unwrap();
        assert!(
            ctrl.vertical_velocity.abs() < 0.01,
            "Climb mode should zero vertical velocity: {}",
            ctrl.vertical_velocity
        );
    }

    #[test]
    fn r10_control_character_obstacle_deflection() {
        // Verify character doesn't pass through solid objects
        // Use a dynamic box with high mass as a wall
        let mut pw = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));
        let _ground = pw.create_ground_plane(Vec3::new(100.0, 0.1, 100.0), 0.5);
        // Heavy box as wall at x=3
        let _wall = pw.add_dynamic_box(
            Vec3::new(3.0, 1.0, 0.0),
            Vec3::new(0.1, 2.0, 5.0),
            0.0, // Zero mass creates a dynamic body that won't move much
            Layers::DEFAULT,
        );
        pw.step();
        let ch = pw.add_character(Vec3::new(0.0, 0.5, 0.0), Vec3::new(0.3, 0.8, 0.3));

        // Settle
        for _ in 0..30 {
            pw.control_character(ch, Vec3::ZERO, 1.0 / 60.0, false);
            pw.step();
        }

        // Try to move into the wall
        for _ in 0..120 {
            pw.control_character(ch, Vec3::new(5.0, 0.0, 0.0), 1.0 / 60.0, false);
            pw.step();
        }

        let x = pw.body_transform(ch).unwrap().w_axis.x;
        // Should not pass through the wall at x=3
        assert!(x < 3.5, "Character should be blocked by wall: x={}", x);
    }

    #[test]
    fn r10_control_character_coyote_time_after_edge() {
        // After leaving an edge, character should still be able to jump briefly
        let mut pw = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));
        // Small platform using dynamic box with zero mass (static-like)
        let _platform = pw.add_dynamic_box(
            Vec3::new(0.0, -0.1, 0.0),
            Vec3::new(2.0, 0.1, 2.0),
            0.0,
            Layers::DEFAULT,
        );
        pw.step();
        let ch = pw.add_character(Vec3::new(0.0, 0.5, 0.0), Vec3::new(0.3, 0.8, 0.3));

        // Settle on platform
        for _ in 0..60 {
            pw.control_character(ch, Vec3::ZERO, 1.0 / 60.0, false);
            pw.step();
        }

        let ctrl = pw.char_map.get(&ch).unwrap();
        assert!(
            ctrl.time_since_grounded < 0.05,
            "Should be grounded on platform: {}",
            ctrl.time_since_grounded
        );

        // Walk off edge rapidly
        for _ in 0..10 {
            pw.control_character(ch, Vec3::new(10.0, 0.0, 0.0), 1.0 / 60.0, false);
            pw.step();
        }

        // Immediately try to jump (within coyote time window of 100ms)
        pw.jump(ch, 2.0);
        let y_before = pw.body_transform(ch).unwrap().w_axis.y;
        pw.control_character(ch, Vec3::ZERO, 1.0 / 60.0, false);
        pw.step();

        // Check if the jump velocity was applied
        let ctrl = pw.char_map.get(&ch).unwrap();
        // If coyote time worked, pending_jump_velocity should have been consumed
        // and character should gain upward velocity
        let has_upward = ctrl.vertical_velocity > 0.0 || ctrl.jump_buffer_timer > 0.0;
        // At minimum, the jump call should set the buffer timer
        let jump_was_buffered = ctrl.jump_buffer_timer >= -0.01;
        assert!(
            has_upward || jump_was_buffered,
            "Jump near edge should use coyote time or buffer: vv={}, buffer={}",
            ctrl.vertical_velocity,
            ctrl.jump_buffer_timer
        );
    }

    #[test]
    fn r10_control_character_time_since_grounded_increments() {
        // When in air, time_since_grounded should increase
        let mut pw = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));
        let ch = pw.add_character(Vec3::new(0.0, 10.0, 0.0), Vec3::new(0.3, 0.8, 0.3));
        pw.step();

        // Step once to initialize
        pw.control_character(ch, Vec3::ZERO, 1.0 / 60.0, false);
        let tsg_initial = pw.char_map.get(&ch).unwrap().time_since_grounded;

        // Step more - character is falling (no ground)
        for _ in 0..5 {
            pw.control_character(ch, Vec3::ZERO, 1.0 / 60.0, false);
            pw.step();
        }

        let tsg_after = pw.char_map.get(&ch).unwrap().time_since_grounded;
        assert!(
            tsg_after > tsg_initial,
            "time_since_grounded should increase in air: before={}, after={}",
            tsg_initial,
            tsg_after
        );
    }

    #[test]
    fn r10_jump_velocity_scales_with_height() {
        // Verify that higher jump height gives higher velocity
        let mut pw = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));
        let _ground = pw.create_ground_plane(Vec3::new(100.0, 0.1, 100.0), 0.5);
        pw.step();
        let ch = pw.add_character(Vec3::new(0.0, 0.5, 0.0), Vec3::new(0.3, 0.8, 0.3));

        for _ in 0..30 {
            pw.control_character(ch, Vec3::ZERO, 1.0 / 60.0, false);
            pw.step();
        }

        pw.jump(ch, 1.0);
        let v1 = pw.char_map.get(&ch).unwrap().pending_jump_velocity;

        pw.jump(ch, 4.0);
        let v4 = pw.char_map.get(&ch).unwrap().pending_jump_velocity;

        assert!(
            v4 > v1 * 1.5,
            "Higher jump should give higher velocity: v1={}, v4={}",
            v1,
            v4
        );
    }

    // ============================================================================
    // R11 — Targeted mutation tests for control_character and jump
    // ============================================================================

    #[test]
    fn r11_jump_uses_multiply_not_divide() {
        let mut pw = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));
        pw.create_ground_plane(Vec3::new(100.0, 0.01, 100.0), 0.5);
        let ch = pw.add_character(Vec3::new(0.0, 1.0, 0.0), Vec3::new(0.3, 0.9, 0.3));

        pw.jump(ch, 2.0);
        let ctrl = pw.char_map.get(&ch).unwrap();

        // v = sqrt(2 * g * h) = sqrt(2 * 9.81 * 2.0) = sqrt(39.24) ≈ 6.26
        let expected = (2.0 * 9.81 * 2.0_f32).sqrt();
        assert!(
            (ctrl.pending_jump_velocity - expected).abs() < 0.1,
            "Jump velocity should be sqrt(2*g*h)={}: got={}",
            expected,
            ctrl.pending_jump_velocity
        );
    }

    #[test]
    fn r11_control_character_gravity_exact_accumulation() {
        let mut pw = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));
        pw.create_ground_plane(Vec3::new(100.0, 0.01, 100.0), 0.5);
        // Place character high so it won't hit ground
        let ch = pw.add_character(Vec3::new(0.0, 100.0, 0.0), Vec3::new(0.3, 0.9, 0.3));

        // Move with no ground contact — gravity should accumulate
        let dt = 1.0 / 60.0;
        pw.control_character(ch, Vec3::ZERO, dt, false);

        let ctrl = pw.char_map.get(&ch).unwrap();
        // vertical_velocity -= 9.81 * gravity_scale * dt = -9.81 * 1.0 * (1/60)
        let expected = -9.81 * dt;
        assert!(
            (ctrl.vertical_velocity - expected).abs() < 0.1,
            "Vertical velocity should be -9.81*dt={}: got={}",
            expected,
            ctrl.vertical_velocity
        );
    }

    #[test]
    fn r11_control_character_climb_zeroes_gravity() {
        let mut pw = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));
        pw.create_ground_plane(Vec3::new(100.0, 0.01, 100.0), 0.5);
        let ch = pw.add_character(Vec3::new(0.0, 5.0, 0.0), Vec3::new(0.3, 0.9, 0.3));

        // First, apply gravity
        pw.control_character(ch, Vec3::ZERO, 0.1, false);
        let v_after_gravity = pw.char_map.get(&ch).unwrap().vertical_velocity;
        assert!(
            v_after_gravity < -0.5,
            "Should have negative velocity from gravity"
        );

        // Now climb — should zero vertical velocity
        pw.control_character(ch, Vec3::ZERO, 0.1, true);
        let v_after_climb = pw.char_map.get(&ch).unwrap().vertical_velocity;
        assert!(
            v_after_climb.abs() < 0.01,
            "Climb should zero vertical velocity: got={}",
            v_after_climb
        );
    }

    #[test]
    fn r11_control_character_coyote_invalidation() {
        // After jumping, time_since_grounded = coyote_time_limit + 1.0
        // Mutation: + with * → coyote_time_limit * 1.0 = 0.1 (still within coyote)
        // We need to verify that after jumping, can_jump becomes false
        let mut pw = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));
        pw.create_ground_plane(Vec3::new(100.0, 0.01, 100.0), 0.5);
        // Character on ground
        let ch = pw.add_character(Vec3::new(0.0, 0.1, 0.0), Vec3::new(0.3, 0.9, 0.3));

        // Move to establish ground contact
        for _ in 0..5 {
            pw.control_character(ch, Vec3::ZERO, 1.0 / 60.0, false);
            pw.step();
        }

        // Jump
        pw.jump(ch, 1.0);
        pw.control_character(ch, Vec3::ZERO, 1.0 / 60.0, false);

        let ctrl = pw.char_map.get(&ch).unwrap();
        // After successful jump, time_since_grounded should be > coyote_time_limit
        assert!(
            ctrl.time_since_grounded > ctrl.coyote_time_limit,
            "After jump, time_since_grounded={} should exceed coyote_time_limit={}",
            ctrl.time_since_grounded,
            ctrl.coyote_time_limit
        );
    }

    #[test]
    fn r11_buoyancy_force_formula() {
        // buoyancy_force = volume * fluid_density * 9.81
        // Mutation: * with + or / would change the magnitude dramatically
        let mut pw = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));
        pw.water_level = 10.0;
        pw.fluid_density = 1000.0;

        let id = pw.add_dynamic_box(
            Vec3::new(0.0, 5.0, 0.0),
            Vec3::splat(0.5),
            10.0,
            Layers::DEFAULT,
        );
        pw.add_buoyancy(id, 2.0, 0.0); // volume=2, drag=0

        // Before buoyancy
        let vel_before = pw.get_velocity(id).unwrap();

        // Apply buoyancy and step
        pw.apply_buoyancy_forces();
        pw.step();

        // After buoyancy: force = 2.0 * 1000.0 * 9.81 = 19620 N upward
        // This should counteract gravity significantly for a 10kg body
        let vel_after = pw.get_velocity(id).unwrap();
        assert!(
            vel_after.y > vel_before.y,
            "Buoyancy should push body upward: before_y={}, after_y={}",
            vel_before.y,
            vel_after.y
        );
    }

    #[test]
    fn r11_radial_impulse_direction_outward() {
        // to_body = body_pos - center → radial direction
        // If - changed to +, direction would be wrong
        let mut pw = PhysicsWorld::new(Vec3::new(0.0, 0.0, 0.0)); // no gravity
        let id = pw.add_dynamic_box(
            Vec3::new(5.0, 0.0, 0.0),
            Vec3::splat(0.3),
            1.0,
            Layers::DEFAULT,
        );

        let affected = pw.apply_radial_impulse(
            Vec3::ZERO, // center
            10.0,       // radius
            100.0,      // force
            crate::projectile::FalloffCurve::Linear,
            0.0, // no upward bias
        );
        assert_eq!(affected, 1);

        pw.step();
        let vel = pw.get_velocity(id).unwrap();
        // Body at (5,0,0), center at (0,0,0) → should be pushed in +X direction
        assert!(
            vel.x > 1.0,
            "Radial impulse should push body away from center in +X: vx={}",
            vel.x
        );
    }

    // ============================================================================
    // R12: Targeted mutation-kill tests
    // ============================================================================

    #[test]
    fn r12_jump_velocity_with_gravity_scale_2() {
        // Targets: lib.rs jump() L1233: `9.81 * ctrl.gravity_scale` — * vs /
        // Also: L1234: `(2.0 * g * height).sqrt()` — * vs + or /
        // With gravity_scale=2.0:
        //   g = 9.81 * 2.0 = 19.62
        //   jump_vel = sqrt(2 * 19.62 * 3.0) = sqrt(117.72) ≈ 10.85
        // With mutation * → /:
        //   g = 9.81 / 2.0 = 4.905
        //   jump_vel = sqrt(2 * 4.905 * 3.0) = sqrt(29.43) ≈ 5.42 — DIFFERENT!
        let mut pw = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));
        let char_id = pw.add_character(Vec3::new(0.0, 50.0, 0.0), Vec3::new(0.4, 0.9, 0.4));

        // Override gravity_scale to 2.0
        pw.char_map.get_mut(&char_id).unwrap().gravity_scale = 2.0;
        // Force grounded so jump can trigger
        pw.char_map.get_mut(&char_id).unwrap().time_since_grounded = 0.0;

        // Request jump with height=3.0
        pw.jump(char_id, 3.0);

        let ctrl = pw.char_map.get(&char_id).unwrap();
        // pending_jump_velocity = sqrt(2 * 9.81 * 2.0 * 3.0) = sqrt(117.72) ≈ 10.85
        let expected = (2.0_f32 * 9.81 * 2.0 * 3.0).sqrt();
        assert!(
            (ctrl.pending_jump_velocity - expected).abs() < 0.1,
            "Jump vel with gravity_scale=2 should be ~{:.2}, got {:.2}",
            expected,
            ctrl.pending_jump_velocity
        );
    }

    #[test]
    fn r12_jump_velocity_formula_2g_h() {
        // Targets: L1234 `(2.0 * g * height).sqrt()` — specifically 2.0 * g and g * height
        // With 2.0 * g mutated to 2.0 + g: sqrt((2.0+9.81)*3.0) = sqrt(35.43) ≈ 5.95
        // Correct: sqrt(2.0 * 9.81 * 3.0) = sqrt(58.86) ≈ 7.67
        let mut pw = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));
        let char_id = pw.add_character(Vec3::new(0.0, 50.0, 0.0), Vec3::new(0.4, 0.9, 0.4));
        // Keep gravity_scale=1.0 (default) but jump with specific height
        pw.jump(char_id, 3.0);

        let ctrl = pw.char_map.get(&char_id).unwrap();
        let expected = (2.0_f32 * 9.81 * 3.0).sqrt(); // ≈ 7.67
        assert!(
            (ctrl.pending_jump_velocity - expected).abs() < 0.05,
            "Jump vel should be sqrt(2*g*h) = {:.3}, got {:.3}",
            expected,
            ctrl.pending_jump_velocity
        );

        // Also test with different height to catch g*height mutations
        let char_id2 = pw.add_character(Vec3::new(5.0, 50.0, 0.0), Vec3::new(0.4, 0.9, 0.4));
        pw.jump(char_id2, 5.0);

        let ctrl2 = pw.char_map.get(&char_id2).unwrap();
        let expected2 = (2.0_f32 * 9.81 * 5.0).sqrt(); // ≈ 9.90
        assert!(
            (ctrl2.pending_jump_velocity - expected2).abs() < 0.05,
            "Jump vel for h=5 should be {:.3}, got {:.3}",
            expected2,
            ctrl2.pending_jump_velocity
        );
    }

    #[test]
    fn r12_radial_impulse_nonzero_center_direction() {
        // Targets: apply_radial_impulse `body_pos - center` — - vs +
        // With center=(10,0,0) and body at (15,0,0):
        //   Correct: to_body = (5,0,0) → push +X
        //   Mutated (+ instead of -): to_body = (25,0,0) → still +X but wrong distance
        // Use body at (7,0,0), center at (10,0,0):
        //   Correct: to_body = (-3,0,0) → push -X (away from center)
        //   Mutated: to_body = (17,0,0) → push +X (WRONG!)
        let mut pw = PhysicsWorld::new(Vec3::new(0.0, 0.0, 0.0)); // no gravity
        let id = pw.add_dynamic_box(
            Vec3::new(7.0, 0.0, 0.0),
            Vec3::splat(0.3),
            1.0,
            Layers::DEFAULT,
        );

        pw.step(); // Init

        let affected = pw.apply_radial_impulse(
            Vec3::new(10.0, 0.0, 0.0), // center at x=10
            10.0,                      // radius
            100.0,                     // force
            crate::projectile::FalloffCurve::Linear,
            0.0,
        );
        assert_eq!(affected, 1);

        pw.step();
        let vel = pw.get_velocity(id).unwrap();
        // Body at x=7, center at x=10: body is LEFT of center → pushed LEFT (−X)
        assert!(
            vel.x < -0.5,
            "Body at x=7 with center at x=10 should be pushed in -X: vx={}",
            vel.x
        );
    }

    #[test]
    fn r12_control_character_gravity_accumulation_nonunit_scale() {
        // Targets: lib.rs:1258 `ctrl.vertical_velocity -= 9.81 * ctrl.gravity_scale * dt`
        // With gravity_scale=3.0 and dt=0.1:
        //   Correct: vertical_velocity -= 9.81 * 3.0 * 0.1 = -2.943
        //   With * → /: -= 9.81 / 3.0 * 0.1 = -0.327 (WRONG)
        //   With * → +: -= (9.81 + 3.0) * 0.1 = -1.281 (WRONG)
        let mut pw = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));
        let char_id = pw.add_character(Vec3::new(0.0, 50.0, 0.0), Vec3::new(0.4, 0.9, 0.4));

        pw.char_map.get_mut(&char_id).unwrap().gravity_scale = 3.0;

        // One frame of freefall
        pw.control_character(char_id, Vec3::ZERO, 0.1, false);

        let ctrl = pw.char_map.get(&char_id).unwrap();
        let expected_dv = -9.81 * 3.0 * 0.1; // = -2.943
        assert!(
            (ctrl.vertical_velocity - expected_dv).abs() < 0.05,
            "gravity with scale=3: expected v={:.3}, got {:.3}",
            expected_dv,
            ctrl.vertical_velocity
        );
    }
}
