//! Tool Sandbox: Validated action verbs and error taxonomy

#[cfg(feature = "profiling")]
use astraweave_profiling::span;

use anyhow::Result;
use astraweave_core::{IVec2, WorldSnapshot};
use astraweave_nav::NavMesh;
use rapier3d::prelude::*;

/// Enumeration of all validated action verbs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ToolVerb {
    MoveTo,
    Throw,
    CoverFire,
    Revive,
    Interact,
    UseItem,
    Stay,
    Wander,
    Hide,
    Rally,
}

impl std::fmt::Display for ToolVerb {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ToolVerb::MoveTo => write!(f, "MoveTo"),
            ToolVerb::Throw => write!(f, "Throw"),
            ToolVerb::CoverFire => write!(f, "CoverFire"),
            ToolVerb::Revive => write!(f, "Revive"),
            ToolVerb::Interact => write!(f, "Interact"),
            ToolVerb::UseItem => write!(f, "UseItem"),
            ToolVerb::Stay => write!(f, "Stay"),
            ToolVerb::Wander => write!(f, "Wander"),
            ToolVerb::Hide => write!(f, "Hide"),
            ToolVerb::Rally => write!(f, "Rally"),
        }
    }
}

impl ToolVerb {
    /// Check if this verb is a movement-related action.
    #[must_use]
    pub fn is_movement(&self) -> bool {
        matches!(
            self,
            ToolVerb::MoveTo | ToolVerb::Wander | ToolVerb::Hide | ToolVerb::Stay
        )
    }

    /// Check if this verb is a combat-related action.
    #[must_use]
    pub fn is_combat(&self) -> bool {
        matches!(self, ToolVerb::Throw | ToolVerb::CoverFire)
    }

    /// Check if this verb is a support action.
    #[must_use]
    pub fn is_support(&self) -> bool {
        matches!(self, ToolVerb::Revive | ToolVerb::Rally)
    }

    /// Check if this verb requires a target position.
    #[must_use]
    pub fn requires_target_position(&self) -> bool {
        matches!(
            self,
            ToolVerb::MoveTo
                | ToolVerb::Throw
                | ToolVerb::CoverFire
                | ToolVerb::Revive
                | ToolVerb::Hide
        )
    }

    /// Check if this verb requires ammo.
    #[must_use]
    pub fn requires_ammo(&self) -> bool {
        matches!(self, ToolVerb::Throw | ToolVerb::CoverFire)
    }

    /// Check if this verb requires line of sight validation.
    #[must_use]
    pub fn requires_line_of_sight(&self) -> bool {
        matches!(self, ToolVerb::Throw | ToolVerb::CoverFire)
    }

    /// Get the primary validation category for this verb.
    #[must_use]
    pub fn primary_validation_category(&self) -> ValidationCategory {
        match self {
            ToolVerb::MoveTo | ToolVerb::Wander | ToolVerb::Hide => ValidationCategory::Nav,
            ToolVerb::Throw | ToolVerb::CoverFire => ValidationCategory::Visibility,
            ToolVerb::Revive | ToolVerb::UseItem => ValidationCategory::Resources,
            ToolVerb::Interact => ValidationCategory::Physics,
            ToolVerb::Stay | ToolVerb::Rally => ValidationCategory::Cooldown,
        }
    }

    /// Get all tool verbs.
    #[must_use]
    pub fn all() -> &'static [ToolVerb] {
        &[
            ToolVerb::MoveTo,
            ToolVerb::Throw,
            ToolVerb::CoverFire,
            ToolVerb::Revive,
            ToolVerb::Interact,
            ToolVerb::UseItem,
            ToolVerb::Stay,
            ToolVerb::Wander,
            ToolVerb::Hide,
            ToolVerb::Rally,
        ]
    }
}

/// Validation categories for each verb
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ValidationCategory {
    Nav,
    Physics,
    Resources,
    Visibility,
    Cooldown,
}

impl std::fmt::Display for ValidationCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValidationCategory::Nav => write!(f, "Nav"),
            ValidationCategory::Physics => write!(f, "Physics"),
            ValidationCategory::Resources => write!(f, "Resources"),
            ValidationCategory::Visibility => write!(f, "Visibility"),
            ValidationCategory::Cooldown => write!(f, "Cooldown"),
        }
    }
}

impl ValidationCategory {
    /// Get all validation categories.
    #[must_use]
    pub fn all() -> &'static [ValidationCategory] {
        &[
            ValidationCategory::Nav,
            ValidationCategory::Physics,
            ValidationCategory::Resources,
            ValidationCategory::Visibility,
            ValidationCategory::Cooldown,
        ]
    }

    /// Check if this category requires external systems (nav mesh or physics).
    #[must_use]
    pub fn requires_external_system(&self) -> bool {
        matches!(self, ValidationCategory::Nav | ValidationCategory::Physics)
    }
}

/// Error taxonomy for tool validation
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
#[must_use]
pub enum ToolError {
    OutOfBounds,
    Cooldown,
    NoLineOfSight,
    InsufficientResource,
    InvalidTarget,
    PhysicsBlocked,
    NoPath,
    Unknown,
}

use std::fmt;
impl fmt::Display for ToolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            ToolError::OutOfBounds => "OutOfBounds",
            ToolError::Cooldown => "Cooldown",
            ToolError::NoLineOfSight => "NoLineOfSight",
            ToolError::InsufficientResource => "InsufficientResource",
            ToolError::InvalidTarget => "InvalidTarget",
            ToolError::PhysicsBlocked => "PhysicsBlocked",
            ToolError::NoPath => "NoPath",
            ToolError::Unknown => "Unknown",
        };
        write!(f, "{}", s)
    }
}

impl ToolError {
    /// Check if this error is related to navigation.
    #[must_use]
    pub fn is_nav_error(&self) -> bool {
        matches!(self, ToolError::OutOfBounds | ToolError::NoPath)
    }

    /// Check if this error is related to physics.
    #[must_use]
    pub fn is_physics_error(&self) -> bool {
        matches!(self, ToolError::PhysicsBlocked)
    }

    /// Check if this error is related to resources.
    #[must_use]
    pub fn is_resource_error(&self) -> bool {
        matches!(self, ToolError::InsufficientResource)
    }

    /// Check if this error is related to visibility.
    #[must_use]
    pub fn is_visibility_error(&self) -> bool {
        matches!(self, ToolError::NoLineOfSight)
    }

    /// Check if this error is related to targeting.
    #[must_use]
    pub fn is_target_error(&self) -> bool {
        matches!(self, ToolError::InvalidTarget)
    }

    /// Check if this error is a timing/cooldown error.
    #[must_use]
    pub fn is_timing_error(&self) -> bool {
        matches!(self, ToolError::Cooldown)
    }

    /// Check if this is a recoverable error (can retry after some time).
    #[must_use]
    pub fn is_recoverable(&self) -> bool {
        matches!(self, ToolError::Cooldown | ToolError::NoLineOfSight)
    }

    /// Get the validation category this error belongs to.
    #[must_use]
    pub fn category(&self) -> ValidationCategory {
        match self {
            ToolError::OutOfBounds | ToolError::NoPath => ValidationCategory::Nav,
            ToolError::PhysicsBlocked => ValidationCategory::Physics,
            ToolError::InsufficientResource => ValidationCategory::Resources,
            ToolError::NoLineOfSight => ValidationCategory::Visibility,
            ToolError::InvalidTarget => ValidationCategory::Resources,
            ToolError::Cooldown => ValidationCategory::Cooldown,
            ToolError::Unknown => ValidationCategory::Resources,
        }
    }

    /// Get all tool errors.
    #[must_use = "returns the complete list of known tool errors"]
    pub fn all() -> &'static [ToolError] {
        &[
            ToolError::OutOfBounds,
            ToolError::Cooldown,
            ToolError::NoLineOfSight,
            ToolError::InsufficientResource,
            ToolError::InvalidTarget,
            ToolError::PhysicsBlocked,
            ToolError::NoPath,
            ToolError::Unknown,
        ]
    }
}

/// Extended validation context with navmesh and physics
pub struct ValidationContext<'a> {
    pub nav_mesh: Option<&'a NavMesh>,
    pub physics_pipeline: Option<&'a PhysicsPipeline>,
    pub rigid_body_set: Option<&'a RigidBodySet>,
    pub collider_set: Option<&'a ColliderSet>,
}

impl<'a> Default for ValidationContext<'a> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> ValidationContext<'a> {
    pub fn new() -> Self {
        Self {
            nav_mesh: None,
            physics_pipeline: None,
            rigid_body_set: None,
            collider_set: None,
        }
    }

    pub fn with_nav(mut self, nav: &'a NavMesh) -> Self {
        self.nav_mesh = Some(nav);
        self
    }

    pub fn with_physics(
        mut self,
        pipeline: &'a PhysicsPipeline,
        bodies: &'a RigidBodySet,
        colliders: &'a ColliderSet,
    ) -> Self {
        self.physics_pipeline = Some(pipeline);
        self.rigid_body_set = Some(bodies);
        self.collider_set = Some(colliders);
        self
    }

    /// Check if nav mesh is available for path validation.
    #[must_use]
    pub fn has_nav(&self) -> bool {
        self.nav_mesh.is_some()
    }

    /// Check if physics is available for collision validation.
    #[must_use]
    pub fn has_physics(&self) -> bool {
        self.physics_pipeline.is_some()
            && self.rigid_body_set.is_some()
            && self.collider_set.is_some()
    }

    /// Check if all validation systems are available.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.has_nav() && self.has_physics()
    }

    /// Check if validation context is empty (no systems available).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        !self.has_nav() && !self.has_physics()
    }

    /// Get a list of available validation categories.
    #[must_use]
    pub fn available_categories(&self) -> Vec<ValidationCategory> {
        let mut categories = vec![
            ValidationCategory::Resources,
            ValidationCategory::Visibility,
            ValidationCategory::Cooldown,
        ];
        if self.has_nav() {
            categories.push(ValidationCategory::Nav);
        }
        if self.has_physics() {
            categories.push(ValidationCategory::Physics);
        }
        categories
    }
}

/// Validate ammo availability
fn validate_ammo(world: &WorldSnapshot) -> Result<()> {
    if world.me.ammo == 0 {
        anyhow::bail!("action blocked: insufficient ammo");
    }
    Ok(())
}
/// Validate line of sight to target position
fn validate_line_of_sight(world: &WorldSnapshot, target_pos: Option<IVec2>) -> Result<()> {
    if let Some(pos) = target_pos {
        if !has_line_of_sight(world.me.pos, pos, world) {
            anyhow::bail!("action blocked: no line of sight");
        }
    }
    Ok(())
}

/// Validate a tool action for an agent in the world
pub fn validate_tool_action(
    _agent_id: u32,
    verb: ToolVerb,
    world: &WorldSnapshot,
    context: &ValidationContext,
    target_pos: Option<IVec2>,
) -> Result<()> {
    #[cfg(feature = "profiling")]
    span!("AI::ToolSandbox::validate");

    // Cooldown checks
    if let Some(cd) = world
        .me
        .cooldowns
        .get(&format!("{:?}", verb).to_lowercase())
    {
        if *cd > 0.0 {
            return Err(anyhow::anyhow!(
                "action blocked for verb {:?}: cooldown {:.2}",
                verb,
                cd
            ));
        }
    }
    match verb {
        ToolVerb::MoveTo => {
            if let Some(pos) = target_pos {
                // Nav check: path exists
                if let Some(nav) = context.nav_mesh {
                    let start = glam::Vec3::new(world.me.pos.x as f32, 0.0, world.me.pos.y as f32);
                    let goal = glam::Vec3::new(pos.x as f32, 0.0, pos.y as f32);
                    if nav.find_path(start, goal).is_empty() {
                        anyhow::bail!(
                            "action blocked: no path from {:?} to {:?}",
                            world.me.pos,
                            pos
                        );
                    }
                }
                // Physics check: not blocked by colliders
                if let (Some(_pipeline), Some(bodies), Some(colliders)) = (
                    context.physics_pipeline,
                    context.rigid_body_set,
                    context.collider_set,
                ) {
                    // Check if target position has overlapping colliders
                    // Create a small query AABB at the target position (assuming 2D movement on Y=0 plane)
                    let half_extents = vector![0.4, 0.1, 0.4]; // Small bounding box for agent
                    let query_pos = point![pos.x as f32, 0.0, pos.y as f32];
                    let query_aabb = Aabb::from_half_extents(query_pos, half_extents);

                    // Query for intersecting colliders, but exclude dynamic bodies (agents)
                    // Only consider static/environment colliders as obstacles
                    let mut intersecting = false;
                    for (_collider_handle, collider) in colliders.iter() {
                        // Skip colliders attached to dynamic rigid bodies (agents)
                        if let Some(parent_handle) = collider.parent() {
                            if let Some(rigid_body) = bodies.get(parent_handle) {
                                // Only consider static rigid bodies as obstacles
                                if rigid_body.is_dynamic() {
                                    continue;
                                }
                            }
                        }

                        let collider_aabb = collider.compute_aabb();
                        if query_aabb.intersects(&collider_aabb) {
                            intersecting = true;
                            break;
                        }
                    }

                    if intersecting {
                        anyhow::bail!("action blocked: physics collision at position {:?}", pos);
                    }
                }
            }
        }
        ToolVerb::Throw => {
            validate_ammo(world)?;
            validate_line_of_sight(world, target_pos)?;
        }
        ToolVerb::CoverFire => {
            validate_ammo(world)?;
            validate_line_of_sight(world, target_pos)?;
        }
        ToolVerb::Revive => {
            // Resources: stamina or something
            if world.me.morale < 0.5 {
                anyhow::bail!(
                    "action blocked: low morale for Revive (morale: {:.2}, required: 0.5)",
                    world.me.morale
                );
            }
            // Target valid: ally nearby
            if let Some(pos) = target_pos {
                let dx = pos.x - world.me.pos.x;
                let dy = pos.y - world.me.pos.y;
                let dist = ((dx * dx + dy * dy) as f32).sqrt();
                if dist > 2.0 {
                    anyhow::bail!(
                        "action blocked: target too far for Revive (distance: {:.2}, max: 2.0)",
                        dist
                    );
                }
            }
        }
        _ => {} // Other actions OK for now
    }
    Ok(())
}

/// Simple LOS check (placeholder: check obstacles)
fn has_line_of_sight(from: IVec2, to: IVec2, world: &WorldSnapshot) -> bool {
    // Bresenham line, check if any obstacle intersects
    let dx = (to.x - from.x).abs();
    let dy = (to.y - from.y).abs();
    let sx = if from.x < to.x { 1 } else { -1 };
    let sy = if from.y < to.y { 1 } else { -1 };
    let mut err = dx - dy;

    let mut x = from.x;
    let mut y = from.y;

    while x != to.x || y != to.y {
        if world.obstacles.iter().any(|obs| obs.x == x && obs.y == y) {
            return false;
        }
        let e2 = 2 * err;
        if e2 > -dy {
            err -= dy;
            x += sx;
        }
        if e2 < dx {
            err += dx;
            y += sy;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use astraweave_core::{CompanionState, PlayerState, WorldSnapshot};

    #[test]
    fn error_taxonomy_works() {
        assert_eq!(ToolError::OutOfBounds.to_string(), "OutOfBounds");
        assert_eq!(ToolError::NoPath.to_string(), "NoPath");
        assert_eq!(ToolError::NoLineOfSight.to_string(), "NoLineOfSight");
        assert_eq!(
            ToolError::InsufficientResource.to_string(),
            "InsufficientResource"
        );
        assert_eq!(ToolError::Cooldown.to_string(), "Cooldown");
        assert_eq!(ToolError::InvalidTarget.to_string(), "InvalidTarget");
        assert_eq!(ToolError::PhysicsBlocked.to_string(), "PhysicsBlocked");
        assert_eq!(ToolError::Unknown.to_string(), "Unknown");
    }

    #[test]
    fn validate_move_to_no_path() {
        let world = WorldSnapshot {
            t: 0.0,
            player: PlayerState {
                hp: 100,

                pos: IVec2 { x: 0, y: 0 },
                stance: "standing".into(),
                orders: vec![],
            },
            me: CompanionState {
                ammo: 10,
                cooldowns: std::collections::BTreeMap::new(),
                morale: 1.0,
                pos: IVec2 { x: 0, y: 0 },
            },
            enemies: vec![],
            pois: vec![],
            objective: None,
            obstacles: vec![IVec2 { x: 1, y: 0 }], // obstacle at (1,0)
        };
        let nav = NavMesh::bake(&[], 0.4, 60.0); // empty nav, no path
        let context = ValidationContext::new().with_nav(&nav);
        let result = validate_tool_action(
            0,
            ToolVerb::MoveTo,
            &world,
            &context,
            Some(IVec2 { x: 2, y: 0 }),
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("no path"));
    }

    #[test]
    fn validate_throw_insufficient_ammo() {
        let world = WorldSnapshot {
            t: 0.0,
            player: PlayerState {
                hp: 100,

                pos: IVec2 { x: 0, y: 0 },
                stance: "standing".into(),
                orders: vec![],
            },
            me: CompanionState {
                ammo: 0,
                cooldowns: std::collections::BTreeMap::new(),
                morale: 1.0,
                pos: IVec2 { x: 0, y: 0 },
            },
            enemies: vec![],
            pois: vec![],
            objective: None,
            obstacles: vec![],
        };
        let context = ValidationContext::new();
        let result = validate_tool_action(
            0,
            ToolVerb::Throw,
            &world,
            &context,
            Some(IVec2 { x: 1, y: 0 }),
        );
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("insufficient ammo"));
    }

    #[test]
    fn validate_move_to_physics_blocked() {
        // Create a minimal physics world with a collider at the target position
        let mut rigid_body_set = RigidBodySet::new();
        let mut collider_set = ColliderSet::new();

        // Add a static rigid body with a collider at position (2, 0, 2)
        let rigid_body = RigidBodyBuilder::fixed()
            .translation(vector![2.0, 0.0, 2.0])
            .build();
        let body_handle = rigid_body_set.insert(rigid_body);

        let collider = ColliderBuilder::cuboid(0.5, 0.5, 0.5).build();
        collider_set.insert_with_parent(collider, body_handle, &mut rigid_body_set);

        let physics_pipeline = PhysicsPipeline::new();

        let world = WorldSnapshot {
            t: 0.0,
            player: PlayerState {
                hp: 100,

                pos: IVec2 { x: 0, y: 0 },
                stance: "standing".into(),
                orders: vec![],
            },
            me: CompanionState {
                ammo: 10,
                cooldowns: std::collections::BTreeMap::new(),
                morale: 1.0,
                pos: IVec2 { x: 0, y: 0 },
            },
            enemies: vec![],
            pois: vec![],
            objective: None,
            obstacles: vec![],
        };

        let context = ValidationContext::new().with_physics(
            &physics_pipeline,
            &rigid_body_set,
            &collider_set,
        );
        let result = validate_tool_action(
            0,
            ToolVerb::MoveTo,
            &world,
            &context,
            Some(IVec2 { x: 2, y: 2 }),
        );
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("physics collision"));
    }

    // ===== NEW TESTS (8 tests added) =====

    #[test]
    fn test_cover_fire_insufficient_ammo() {
        // CoverFire requires ammo, should fail when ammo = 0
        let world = WorldSnapshot {
            t: 0.0,
            player: PlayerState {
                hp: 100,

                pos: IVec2 { x: 0, y: 0 },
                stance: "standing".into(),
                orders: vec![],
            },
            me: CompanionState {
                ammo: 0, // No ammo
                cooldowns: std::collections::BTreeMap::new(),
                morale: 1.0,
                pos: IVec2 { x: 0, y: 0 },
            },
            enemies: vec![],
            pois: vec![],
            objective: None,
            obstacles: vec![],
        };

        let context = ValidationContext::new();
        let result = validate_tool_action(
            0,
            ToolVerb::CoverFire,
            &world,
            &context,
            Some(IVec2 { x: 3, y: 3 }),
        );

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("insufficient ammo"));
    }

    #[test]
    fn test_cover_fire_no_line_of_sight() {
        // CoverFire requires LoS, should fail when obstacles block
        let world = WorldSnapshot {
            t: 0.0,
            player: PlayerState {
                hp: 100,

                pos: IVec2 { x: 0, y: 0 },
                stance: "standing".into(),
                orders: vec![],
            },
            me: CompanionState {
                ammo: 10,
                cooldowns: std::collections::BTreeMap::new(),
                morale: 1.0,
                pos: IVec2 { x: 0, y: 0 },
            },
            enemies: vec![],
            pois: vec![],
            objective: None,
            obstacles: vec![IVec2 { x: 1, y: 0 }, IVec2 { x: 2, y: 0 }], // Block path
        };

        let context = ValidationContext::new();
        let result = validate_tool_action(
            0,
            ToolVerb::CoverFire,
            &world,
            &context,
            Some(IVec2 { x: 3, y: 0 }),
        );

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("no line of sight"));
    }

    #[test]
    fn test_cover_fire_success_with_ammo_and_los() {
        // CoverFire should succeed when ammo > 0 and LoS is clear
        let world = WorldSnapshot {
            t: 0.0,
            player: PlayerState {
                hp: 100,

                pos: IVec2 { x: 0, y: 0 },
                stance: "standing".into(),
                orders: vec![],
            },
            me: CompanionState {
                ammo: 10,
                cooldowns: std::collections::BTreeMap::new(),
                morale: 1.0,
                pos: IVec2 { x: 0, y: 0 },
            },
            enemies: vec![],
            pois: vec![],
            objective: None,
            obstacles: vec![], // No obstacles
        };

        let context = ValidationContext::new();
        let result = validate_tool_action(
            0,
            ToolVerb::CoverFire,
            &world,
            &context,
            Some(IVec2 { x: 3, y: 3 }),
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_revive_low_morale() {
        // Revive requires morale >= 0.5, should fail when low
        let world = WorldSnapshot {
            t: 0.0,
            player: PlayerState {
                hp: 100,

                pos: IVec2 { x: 0, y: 0 },
                stance: "standing".into(),
                orders: vec![],
            },
            me: CompanionState {
                ammo: 10,
                cooldowns: std::collections::BTreeMap::new(),
                morale: 0.3, // Too low
                pos: IVec2 { x: 0, y: 0 },
            },
            enemies: vec![],
            pois: vec![],
            objective: None,
            obstacles: vec![],
        };

        let context = ValidationContext::new();
        let result = validate_tool_action(
            0,
            ToolVerb::Revive,
            &world,
            &context,
            Some(IVec2 { x: 1, y: 1 }),
        );

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("low morale"));
    }

    #[test]
    fn test_revive_target_too_far() {
        // Revive requires target within 2.0 distance
        let world = WorldSnapshot {
            t: 0.0,
            player: PlayerState {
                hp: 100,

                pos: IVec2 { x: 0, y: 0 },
                stance: "standing".into(),
                orders: vec![],
            },
            me: CompanionState {
                ammo: 10,
                cooldowns: std::collections::BTreeMap::new(),
                morale: 1.0,
                pos: IVec2 { x: 0, y: 0 },
            },
            enemies: vec![],
            pois: vec![],
            objective: None,
            obstacles: vec![],
        };

        let context = ValidationContext::new();
        let result = validate_tool_action(
            0,
            ToolVerb::Revive,
            &world,
            &context,
            Some(IVec2 { x: 5, y: 5 }), // Distance = ~7.07, exceeds 2.0
        );

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("target too far"));
    }

    #[test]
    fn test_validation_context_builders() {
        // Test ValidationContext builder pattern
        let nav = NavMesh::bake(&[], 0.4, 60.0);

        let rigid_body_set = RigidBodySet::new();
        let collider_set = ColliderSet::new();
        let physics_pipeline = PhysicsPipeline::new();

        // Test with_nav
        let context1 = ValidationContext::new().with_nav(&nav);
        assert!(context1.nav_mesh.is_some());
        assert!(context1.physics_pipeline.is_none());

        // Test with_physics
        let context2 = ValidationContext::new().with_physics(
            &physics_pipeline,
            &rigid_body_set,
            &collider_set,
        );
        assert!(context2.nav_mesh.is_none());
        assert!(context2.physics_pipeline.is_some());
        assert!(context2.rigid_body_set.is_some());
        assert!(context2.collider_set.is_some());

        // Test chaining both
        let context3 = ValidationContext::new().with_nav(&nav).with_physics(
            &physics_pipeline,
            &rigid_body_set,
            &collider_set,
        );
        assert!(context3.nav_mesh.is_some());
        assert!(context3.physics_pipeline.is_some());
    }

    #[test]
    fn test_cooldown_blocking() {
        // Test that cooldowns block actions
        let mut cooldowns = std::collections::BTreeMap::new();
        cooldowns.insert("coverfire".to_string(), 2.5); // CoverFire on cooldown

        let world = WorldSnapshot {
            t: 0.0,
            player: PlayerState {
                hp: 100,

                pos: IVec2 { x: 0, y: 0 },
                stance: "standing".into(),
                orders: vec![],
            },
            me: CompanionState {
                ammo: 10,
                cooldowns,
                morale: 1.0,
                pos: IVec2 { x: 0, y: 0 },
            },
            enemies: vec![],
            pois: vec![],
            objective: None,
            obstacles: vec![],
        };

        let context = ValidationContext::new();
        let result = validate_tool_action(
            0,
            ToolVerb::CoverFire,
            &world,
            &context,
            Some(IVec2 { x: 3, y: 3 }),
        );

        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("cooldown"));
        assert!(err_msg.contains("2.5")); // Cooldown value should be in error message
    }

    #[test]
    fn test_stay_and_wander_always_valid() {
        // Stay and Wander actions should always succeed (no validation checks)
        let world = WorldSnapshot {
            t: 0.0,
            player: PlayerState {
                hp: 100,

                pos: IVec2 { x: 0, y: 0 },
                stance: "standing".into(),
                orders: vec![],
            },
            me: CompanionState {
                ammo: 0, // Even with no ammo
                cooldowns: std::collections::BTreeMap::new(),
                morale: 0.0, // Even with zero morale
                pos: IVec2 { x: 0, y: 0 },
            },
            enemies: vec![],
            pois: vec![],
            objective: None,
            obstacles: vec![],
        };

        let context = ValidationContext::new();

        // Stay should always succeed
        let result_stay = validate_tool_action(0, ToolVerb::Stay, &world, &context, None);
        assert!(result_stay.is_ok());

        // Wander should always succeed
        let result_wander = validate_tool_action(0, ToolVerb::Wander, &world, &context, None);
        assert!(result_wander.is_ok());
    }

    // ========================================
    // Additional ToolVerb Variant Tests
    // ========================================

    #[test]
    fn test_interact_always_valid() {
        // Interact should succeed with minimal world state
        let world = WorldSnapshot {
            t: 0.0,
            player: PlayerState {
                hp: 100,

                pos: IVec2 { x: 0, y: 0 },
                stance: "standing".into(),
                orders: vec![],
            },
            me: CompanionState {
                ammo: 0,
                cooldowns: std::collections::BTreeMap::new(),
                morale: 0.0,
                pos: IVec2 { x: 0, y: 0 },
            },
            enemies: vec![],
            pois: vec![],
            objective: None,
            obstacles: vec![],
        };

        let context = ValidationContext::new();
        let result = validate_tool_action(0, ToolVerb::Interact, &world, &context, None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_use_item_always_valid() {
        // UseItem should succeed with minimal world state
        let world = WorldSnapshot {
            t: 0.0,
            player: PlayerState {
                hp: 100,

                pos: IVec2 { x: 0, y: 0 },
                stance: "standing".into(),
                orders: vec![],
            },
            me: CompanionState {
                ammo: 0,
                cooldowns: std::collections::BTreeMap::new(),
                morale: 0.0,
                pos: IVec2 { x: 0, y: 0 },
            },
            enemies: vec![],
            pois: vec![],
            objective: None,
            obstacles: vec![],
        };

        let context = ValidationContext::new();
        let result = validate_tool_action(0, ToolVerb::UseItem, &world, &context, None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_hide_always_valid() {
        // Hide should succeed with minimal world state
        let world = WorldSnapshot {
            t: 0.0,
            player: PlayerState {
                hp: 100,

                pos: IVec2 { x: 0, y: 0 },
                stance: "standing".into(),
                orders: vec![],
            },
            me: CompanionState {
                ammo: 0,
                cooldowns: std::collections::BTreeMap::new(),
                morale: 0.0,
                pos: IVec2 { x: 0, y: 0 },
            },
            enemies: vec![],
            pois: vec![],
            objective: None,
            obstacles: vec![],
        };

        let context = ValidationContext::new();
        let result = validate_tool_action(0, ToolVerb::Hide, &world, &context, None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_rally_always_valid() {
        // Rally should succeed with minimal world state
        let world = WorldSnapshot {
            t: 0.0,
            player: PlayerState {
                hp: 100,

                pos: IVec2 { x: 0, y: 0 },
                stance: "standing".into(),
                orders: vec![],
            },
            me: CompanionState {
                ammo: 0,
                cooldowns: std::collections::BTreeMap::new(),
                morale: 0.0,
                pos: IVec2 { x: 0, y: 0 },
            },
            enemies: vec![],
            pois: vec![],
            objective: None,
            obstacles: vec![],
        };

        let context = ValidationContext::new();
        let result = validate_tool_action(0, ToolVerb::Rally, &world, &context, None);
        assert!(result.is_ok());
    }

    // ========================================
    // ToolVerb Enum Tests
    // ========================================

    #[test]
    fn test_tool_verb_debug() {
        // Test Debug derive for all variants
        let verbs = vec![
            ToolVerb::MoveTo,
            ToolVerb::Throw,
            ToolVerb::CoverFire,
            ToolVerb::Revive,
            ToolVerb::Interact,
            ToolVerb::UseItem,
            ToolVerb::Stay,
            ToolVerb::Wander,
            ToolVerb::Hide,
            ToolVerb::Rally,
        ];

        for verb in verbs {
            let debug_str = format!("{:?}", verb);
            assert!(!debug_str.is_empty(), "Debug output should not be empty");
        }
    }

    #[test]
    #[allow(clippy::clone_on_copy)]
    fn test_tool_verb_clone_and_copy() {
        // Test Clone and Copy derives
        let verb1 = ToolVerb::MoveTo;
        let verb2 = verb1; // Copy
        let verb3 = verb1.clone(); // Clone

        assert_eq!(verb1, verb2);
        assert_eq!(verb1, verb3);
    }

    #[test]
    fn test_tool_verb_partial_eq() {
        // Test PartialEq
        assert_eq!(ToolVerb::MoveTo, ToolVerb::MoveTo);
        assert_ne!(ToolVerb::MoveTo, ToolVerb::Throw);
        assert_eq!(ToolVerb::CoverFire, ToolVerb::CoverFire);
    }

    #[test]
    fn test_tool_verb_hash() {
        // Test Hash derive (via HashMap insertion)
        use std::collections::HashMap;
        let mut map = HashMap::new();
        map.insert(ToolVerb::MoveTo, 1);
        map.insert(ToolVerb::Throw, 2);
        map.insert(ToolVerb::CoverFire, 3);

        assert_eq!(map.get(&ToolVerb::MoveTo), Some(&1));
        assert_eq!(map.get(&ToolVerb::Throw), Some(&2));
        assert_eq!(map.get(&ToolVerb::CoverFire), Some(&3));
    }

    // ========================================
    // ValidationCategory Enum Tests
    // ========================================

    #[test]
    fn test_validation_category_all_variants() {
        // Test all 5 ValidationCategory variants
        let categories = vec![
            ValidationCategory::Nav,
            ValidationCategory::Physics,
            ValidationCategory::Resources,
            ValidationCategory::Visibility,
            ValidationCategory::Cooldown,
        ];

        for category in categories {
            let debug_str = format!("{:?}", category);
            assert!(!debug_str.is_empty());
        }
    }

    #[test]
    fn test_validation_category_partial_eq() {
        assert_eq!(ValidationCategory::Nav, ValidationCategory::Nav);
        assert_ne!(ValidationCategory::Nav, ValidationCategory::Physics);
        assert_eq!(ValidationCategory::Cooldown, ValidationCategory::Cooldown);
    }

    #[test]
    fn test_validation_category_hash() {
        use std::collections::HashMap;
        let mut map = HashMap::new();
        map.insert(ValidationCategory::Nav, "navigation");
        map.insert(ValidationCategory::Physics, "physics");
        map.insert(ValidationCategory::Resources, "resources");

        assert_eq!(map.get(&ValidationCategory::Nav), Some(&"navigation"));
        assert_eq!(map.get(&ValidationCategory::Physics), Some(&"physics"));
    }

    // ========================================
    // ToolError Enum Tests
    // ========================================

    #[test]
    fn test_tool_error_clone() {
        let err1 = ToolError::OutOfBounds;
        let err2 = err1.clone();
        assert_eq!(err1, err2);
    }

    #[test]
    fn test_tool_error_partial_eq() {
        assert_eq!(ToolError::NoPath, ToolError::NoPath);
        assert_ne!(ToolError::NoPath, ToolError::Cooldown);
    }

    #[test]
    fn test_tool_error_debug() {
        let err = ToolError::InvalidTarget;
        let debug_str = format!("{:?}", err);
        assert!(debug_str.contains("InvalidTarget"));
    }

    // ========================================
    // has_line_of_sight Edge Cases
    // ========================================

    #[test]
    fn test_has_line_of_sight_same_position() {
        // LoS from position to itself should succeed
        let world = WorldSnapshot {
            t: 0.0,
            player: PlayerState {
                hp: 100,

                pos: IVec2 { x: 0, y: 0 },
                stance: "standing".into(),
                orders: vec![],
            },
            me: CompanionState {
                ammo: 10,
                cooldowns: std::collections::BTreeMap::new(),
                morale: 1.0,
                pos: IVec2 { x: 5, y: 5 },
            },
            enemies: vec![],
            pois: vec![],
            objective: None,
            obstacles: vec![],
        };

        let has_los = has_line_of_sight(IVec2 { x: 5, y: 5 }, IVec2 { x: 5, y: 5 }, &world);
        assert!(has_los, "LoS to same position should succeed");
    }

    #[test]
    fn test_has_line_of_sight_horizontal_line() {
        // Horizontal line (y constant)
        let world = WorldSnapshot {
            t: 0.0,
            player: PlayerState {
                hp: 100,

                pos: IVec2 { x: 0, y: 0 },
                stance: "standing".into(),
                orders: vec![],
            },
            me: CompanionState {
                ammo: 10,
                cooldowns: std::collections::BTreeMap::new(),
                morale: 1.0,
                pos: IVec2 { x: 0, y: 0 },
            },
            enemies: vec![],
            pois: vec![],
            objective: None,
            obstacles: vec![], // No obstacles
        };

        let has_los = has_line_of_sight(IVec2 { x: 0, y: 0 }, IVec2 { x: 5, y: 0 }, &world);
        assert!(has_los, "Clear horizontal LoS should succeed");
    }

    #[test]
    fn test_has_line_of_sight_vertical_line() {
        // Vertical line (x constant)
        let world = WorldSnapshot {
            t: 0.0,
            player: PlayerState {
                hp: 100,

                pos: IVec2 { x: 0, y: 0 },
                stance: "standing".into(),
                orders: vec![],
            },
            me: CompanionState {
                ammo: 10,
                cooldowns: std::collections::BTreeMap::new(),
                morale: 1.0,
                pos: IVec2 { x: 0, y: 0 },
            },
            enemies: vec![],
            pois: vec![],
            objective: None,
            obstacles: vec![],
        };

        let has_los = has_line_of_sight(IVec2 { x: 0, y: 0 }, IVec2 { x: 0, y: 5 }, &world);
        assert!(has_los, "Clear vertical LoS should succeed");
    }

    #[test]
    fn test_has_line_of_sight_diagonal_line() {
        // Diagonal line (both x and y change)
        let world = WorldSnapshot {
            t: 0.0,
            player: PlayerState {
                hp: 100,

                pos: IVec2 { x: 0, y: 0 },
                stance: "standing".into(),
                orders: vec![],
            },
            me: CompanionState {
                ammo: 10,
                cooldowns: std::collections::BTreeMap::new(),
                morale: 1.0,
                pos: IVec2 { x: 0, y: 0 },
            },
            enemies: vec![],
            pois: vec![],
            objective: None,
            obstacles: vec![],
        };

        let has_los = has_line_of_sight(IVec2 { x: 0, y: 0 }, IVec2 { x: 5, y: 5 }, &world);
        assert!(has_los, "Clear diagonal LoS should succeed");
    }

    #[test]
    fn test_has_line_of_sight_blocked_midpoint() {
        // Obstacle at midpoint of line
        let world = WorldSnapshot {
            t: 0.0,
            player: PlayerState {
                hp: 100,

                pos: IVec2 { x: 0, y: 0 },
                stance: "standing".into(),
                orders: vec![],
            },
            me: CompanionState {
                ammo: 10,
                cooldowns: std::collections::BTreeMap::new(),
                morale: 1.0,
                pos: IVec2 { x: 0, y: 0 },
            },
            enemies: vec![],
            pois: vec![],
            objective: None,
            obstacles: vec![IVec2 { x: 2, y: 0 }], // Midpoint obstacle
        };

        let has_los = has_line_of_sight(IVec2 { x: 0, y: 0 }, IVec2 { x: 4, y: 0 }, &world);
        assert!(!has_los, "Blocked LoS should fail");
    }

    // ========================================
    // Mutation-Resistant LOS Tests
    // Catch sign flips, operator swaps in Bresenham
    // ========================================

    #[test]
    fn test_has_line_of_sight_reverse_horizontal() {
        // Right-to-left: from (5,0) to (0,0) — tests sx = -1
        let world = WorldSnapshot {
            t: 0.0,
            player: PlayerState {
                hp: 100,
                pos: IVec2 { x: 0, y: 0 },
                stance: "standing".into(),
                orders: vec![],
            },
            me: CompanionState {
                ammo: 10,
                cooldowns: std::collections::BTreeMap::new(),
                morale: 1.0,
                pos: IVec2 { x: 5, y: 0 },
            },
            enemies: vec![],
            pois: vec![],
            objective: None,
            obstacles: vec![],
        };
        assert!(
            has_line_of_sight(IVec2 { x: 5, y: 0 }, IVec2 { x: 0, y: 0 }, &world),
            "Clear reverse horizontal LoS should succeed"
        );

        // Now with obstacle at (3,0) blocking the path
        let world_blocked = WorldSnapshot {
            obstacles: vec![IVec2 { x: 3, y: 0 }],
            ..world
        };
        assert!(
            !has_line_of_sight(IVec2 { x: 5, y: 0 }, IVec2 { x: 0, y: 0 }, &world_blocked),
            "Reverse horizontal LoS with obstacle at (3,0) should fail"
        );
    }

    #[test]
    fn test_has_line_of_sight_reverse_vertical() {
        // Bottom-to-top: from (0,5) to (0,0) — tests sy = -1
        let world = WorldSnapshot {
            t: 0.0,
            player: PlayerState {
                hp: 100,
                pos: IVec2 { x: 0, y: 0 },
                stance: "standing".into(),
                orders: vec![],
            },
            me: CompanionState {
                ammo: 10,
                cooldowns: std::collections::BTreeMap::new(),
                morale: 1.0,
                pos: IVec2 { x: 0, y: 5 },
            },
            enemies: vec![],
            pois: vec![],
            objective: None,
            obstacles: vec![],
        };
        assert!(
            has_line_of_sight(IVec2 { x: 0, y: 5 }, IVec2 { x: 0, y: 0 }, &world),
            "Clear reverse vertical LoS should succeed"
        );

        let world_blocked = WorldSnapshot {
            obstacles: vec![IVec2 { x: 0, y: 2 }],
            ..world
        };
        assert!(
            !has_line_of_sight(IVec2 { x: 0, y: 5 }, IVec2 { x: 0, y: 0 }, &world_blocked),
            "Reverse vertical LoS with obstacle should fail"
        );
    }

    #[test]
    fn test_has_line_of_sight_reverse_diagonal() {
        // From (5,5) to (0,0) — tests sx=-1, sy=-1
        let world = WorldSnapshot {
            t: 0.0,
            player: PlayerState {
                hp: 100,
                pos: IVec2 { x: 0, y: 0 },
                stance: "standing".into(),
                orders: vec![],
            },
            me: CompanionState {
                ammo: 10,
                cooldowns: std::collections::BTreeMap::new(),
                morale: 1.0,
                pos: IVec2 { x: 5, y: 5 },
            },
            enemies: vec![],
            pois: vec![],
            objective: None,
            obstacles: vec![],
        };
        assert!(
            has_line_of_sight(IVec2 { x: 5, y: 5 }, IVec2 { x: 0, y: 0 }, &world),
            "Clear reverse diagonal LoS should succeed"
        );

        let world_blocked = WorldSnapshot {
            obstacles: vec![IVec2 { x: 3, y: 3 }],
            ..world
        };
        assert!(
            !has_line_of_sight(IVec2 { x: 5, y: 5 }, IVec2 { x: 0, y: 0 }, &world_blocked),
            "Reverse diagonal with obstacle at (3,3) should fail"
        );
    }

    #[test]
    fn test_has_line_of_sight_steep_slope() {
        // Non-axis-aligned, non-45°: from (0,0) to (1,4) — steep slope exercises y-stepping
        let world = WorldSnapshot {
            t: 0.0,
            player: PlayerState {
                hp: 100,
                pos: IVec2 { x: 0, y: 0 },
                stance: "standing".into(),
                orders: vec![],
            },
            me: CompanionState {
                ammo: 10,
                cooldowns: std::collections::BTreeMap::new(),
                morale: 1.0,
                pos: IVec2 { x: 0, y: 0 },
            },
            enemies: vec![],
            pois: vec![],
            objective: None,
            obstacles: vec![],
        };
        assert!(
            has_line_of_sight(IVec2 { x: 0, y: 0 }, IVec2 { x: 1, y: 4 }, &world),
            "Clear steep slope LoS should succeed"
        );

        // Obstacle at (0,2) should block the path
        let world_blocked = WorldSnapshot {
            obstacles: vec![IVec2 { x: 0, y: 2 }],
            ..world
        };
        assert!(
            !has_line_of_sight(IVec2 { x: 0, y: 0 }, IVec2 { x: 1, y: 4 }, &world_blocked),
            "Steep slope with obstacle at (0,2) should fail"
        );
    }

    #[test]
    fn test_has_line_of_sight_shallow_slope() {
        // Shallow slope: from (0,0) to (4,1) — exercises x-stepping more than y
        let world = WorldSnapshot {
            t: 0.0,
            player: PlayerState {
                hp: 100,
                pos: IVec2 { x: 0, y: 0 },
                stance: "standing".into(),
                orders: vec![],
            },
            me: CompanionState {
                ammo: 10,
                cooldowns: std::collections::BTreeMap::new(),
                morale: 1.0,
                pos: IVec2 { x: 0, y: 0 },
            },
            enemies: vec![],
            pois: vec![],
            objective: None,
            obstacles: vec![],
        };
        assert!(has_line_of_sight(
            IVec2 { x: 0, y: 0 },
            IVec2 { x: 4, y: 1 },
            &world
        ));

        // Obstacle at (2,0) blocks the horizontal sweep
        let world_blocked = WorldSnapshot {
            obstacles: vec![IVec2 { x: 2, y: 0 }],
            ..world
        };
        assert!(
            !has_line_of_sight(IVec2 { x: 0, y: 0 }, IVec2 { x: 4, y: 1 }, &world_blocked),
            "Shallow slope with obstacle at (2,0) should fail"
        );
    }

    #[test]
    fn test_has_line_of_sight_obstacle_at_start_not_checked() {
        // The start cell itself is not checked for obstacles (Bresenham starts walking from 'from')
        let world = WorldSnapshot {
            t: 0.0,
            player: PlayerState {
                hp: 100,
                pos: IVec2 { x: 0, y: 0 },
                stance: "standing".into(),
                orders: vec![],
            },
            me: CompanionState {
                ammo: 10,
                cooldowns: std::collections::BTreeMap::new(),
                morale: 1.0,
                pos: IVec2 { x: 0, y: 0 },
            },
            enemies: vec![],
            pois: vec![],
            objective: None,
            obstacles: vec![IVec2 { x: 1, y: 0 }], // obstacle at first step
        };
        assert!(
            !has_line_of_sight(IVec2 { x: 0, y: 0 }, IVec2 { x: 3, y: 0 }, &world),
            "Obstacle at first step (1,0) on path to (3,0) should block"
        );
    }

    #[test]
    fn test_has_line_of_sight_adjacent_cells() {
        // Adjacent cells: from (0,0) to (1,0) — single step
        let world_clear = WorldSnapshot {
            t: 0.0,
            player: PlayerState {
                hp: 100,
                pos: IVec2 { x: 0, y: 0 },
                stance: "standing".into(),
                orders: vec![],
            },
            me: CompanionState {
                ammo: 10,
                cooldowns: std::collections::BTreeMap::new(),
                morale: 1.0,
                pos: IVec2 { x: 0, y: 0 },
            },
            enemies: vec![],
            pois: vec![],
            objective: None,
            obstacles: vec![],
        };
        assert!(has_line_of_sight(
            IVec2 { x: 0, y: 0 },
            IVec2 { x: 1, y: 0 },
            &world_clear
        ));
        assert!(has_line_of_sight(
            IVec2 { x: 0, y: 0 },
            IVec2 { x: 0, y: 1 },
            &world_clear
        ));
        assert!(has_line_of_sight(
            IVec2 { x: 0, y: 0 },
            IVec2 { x: 1, y: 1 },
            &world_clear
        ));
    }

    // ========================================
    // Mutation-Resistant Revive Distance Tests
    // ========================================

    #[test]
    fn test_revive_distance_exact_boundary() {
        // Distance exactly 2.0 should pass, > 2.0 should fail
        // sqrt(dx*dx + dy*dy) = 2.0 → dx=2, dy=0
        let make_world = |_pos: IVec2| WorldSnapshot {
            t: 0.0,
            player: PlayerState {
                hp: 100,
                pos: IVec2 { x: 0, y: 0 },
                stance: "standing".into(),
                orders: vec![],
            },
            me: CompanionState {
                ammo: 10,
                cooldowns: std::collections::BTreeMap::new(),
                morale: 1.0,
                pos: IVec2 { x: 0, y: 0 },
            },
            enemies: vec![],
            pois: vec![],
            objective: None,
            obstacles: vec![],
        };
        let context = ValidationContext::new();

        // Distance = sqrt(2*2 + 0*0) = 2.0 → should pass
        let world = make_world(IVec2 { x: 0, y: 0 });
        let result = validate_tool_action(
            0,
            ToolVerb::Revive,
            &world,
            &context,
            Some(IVec2 { x: 2, y: 0 }),
        );
        assert!(result.is_ok(), "Revive at distance 2.0 should pass");

        // Distance = sqrt(2*2 + 1*1) = sqrt(5) ≈ 2.236 → should fail
        let result_far = validate_tool_action(
            0,
            ToolVerb::Revive,
            &world,
            &context,
            Some(IVec2 { x: 2, y: 1 }),
        );
        assert!(result_far.is_err(), "Revive at distance 2.236 should fail");

        // Distance = sqrt(1*1 + 1*1) = sqrt(2) ≈ 1.414 → should pass
        let result_close = validate_tool_action(
            0,
            ToolVerb::Revive,
            &world,
            &context,
            Some(IVec2 { x: 1, y: 1 }),
        );
        assert!(result_close.is_ok(), "Revive at distance 1.414 should pass");
    }

    #[test]
    fn test_revive_distance_arithmetic_correctness() {
        // Verify dx*dx + dy*dy is computed correctly (catches + vs -, * vs + mutations)
        // Me at (3,4), target at (6,8) → dx=3, dy=4, dist=sqrt(9+16)=5.0 → >2.0 → fail
        let world = WorldSnapshot {
            t: 0.0,
            player: PlayerState {
                hp: 100,
                pos: IVec2 { x: 0, y: 0 },
                stance: "standing".into(),
                orders: vec![],
            },
            me: CompanionState {
                ammo: 10,
                cooldowns: std::collections::BTreeMap::new(),
                morale: 1.0,
                pos: IVec2 { x: 3, y: 4 },
            },
            enemies: vec![],
            pois: vec![],
            objective: None,
            obstacles: vec![],
        };
        let context = ValidationContext::new();
        let result = validate_tool_action(
            0,
            ToolVerb::Revive,
            &world,
            &context,
            Some(IVec2 { x: 6, y: 8 }),
        );
        assert!(result.is_err(), "Revive at distance 5.0 should fail (>2.0)");

        // Negative direction: me at (5,5), target at (4,4) → dx=-1, dy=-1, dist=sqrt(2)≈1.41 → pass
        let world2 = WorldSnapshot {
            me: CompanionState {
                pos: IVec2 { x: 5, y: 5 },
                ..world.me.clone()
            },
            ..world.clone()
        };
        let result2 = validate_tool_action(
            0,
            ToolVerb::Revive,
            &world2,
            &context,
            Some(IVec2 { x: 4, y: 4 }),
        );
        assert!(result2.is_ok(), "Revive at distance sqrt(2) should pass");
    }

    // ========================================
    // validate_line_of_sight with None
    // ========================================

    #[test]
    fn test_validate_line_of_sight_none_target() {
        // validate_line_of_sight with None should always pass
        let world = WorldSnapshot {
            t: 0.0,
            player: PlayerState {
                hp: 100,

                pos: IVec2 { x: 0, y: 0 },
                stance: "standing".into(),
                orders: vec![],
            },
            me: CompanionState {
                ammo: 10,
                cooldowns: std::collections::BTreeMap::new(),
                morale: 1.0,
                pos: IVec2 { x: 0, y: 0 },
            },
            enemies: vec![],
            pois: vec![],
            objective: None,
            obstacles: vec![IVec2 { x: 1, y: 0 }], // Obstacles present
        };

        let result = validate_line_of_sight(&world, None);
        assert!(result.is_ok(), "None target should always pass LoS check");
    }

    // ========================================
    // MoveTo with None target
    // ========================================

    #[test]
    fn test_move_to_none_target() {
        // MoveTo with None target should succeed (no validation)
        let world = WorldSnapshot {
            t: 0.0,
            player: PlayerState {
                hp: 100,

                pos: IVec2 { x: 0, y: 0 },
                stance: "standing".into(),
                orders: vec![],
            },
            me: CompanionState {
                ammo: 10,
                cooldowns: std::collections::BTreeMap::new(),
                morale: 1.0,
                pos: IVec2 { x: 0, y: 0 },
            },
            enemies: vec![],
            pois: vec![],
            objective: None,
            obstacles: vec![],
        };

        let context = ValidationContext::new();
        let result = validate_tool_action(0, ToolVerb::MoveTo, &world, &context, None);
        assert!(result.is_ok(), "MoveTo with None target should succeed");
    }

    // ========================================
    // Revive with None target
    // ========================================

    #[test]
    fn test_revive_none_target() {
        // Revive with None target should skip distance check
        let world = WorldSnapshot {
            t: 0.0,
            player: PlayerState {
                hp: 100,

                pos: IVec2 { x: 0, y: 0 },
                stance: "standing".into(),
                orders: vec![],
            },
            me: CompanionState {
                ammo: 10,
                cooldowns: std::collections::BTreeMap::new(),
                morale: 1.0, // High morale
                pos: IVec2 { x: 0, y: 0 },
            },
            enemies: vec![],
            pois: vec![],
            objective: None,
            obstacles: vec![],
        };

        let context = ValidationContext::new();
        let result = validate_tool_action(0, ToolVerb::Revive, &world, &context, None);
        assert!(
            result.is_ok(),
            "Revive with None target should succeed if morale OK"
        );
    }

    // ========================================
    // ValidationContext Default
    // ========================================

    #[test]
    fn test_validation_context_default() {
        let context = ValidationContext::default();
        assert!(context.nav_mesh.is_none());
        assert!(context.physics_pipeline.is_none());
        assert!(context.rigid_body_set.is_none());
        assert!(context.collider_set.is_none());
    }

    // ========================================
    // ToolVerb Display and Helper Tests
    // ========================================

    #[test]
    fn test_tool_verb_display() {
        assert_eq!(format!("{}", ToolVerb::MoveTo), "MoveTo");
        assert_eq!(format!("{}", ToolVerb::Throw), "Throw");
        assert_eq!(format!("{}", ToolVerb::CoverFire), "CoverFire");
        assert_eq!(format!("{}", ToolVerb::Revive), "Revive");
        assert_eq!(format!("{}", ToolVerb::Interact), "Interact");
        assert_eq!(format!("{}", ToolVerb::UseItem), "UseItem");
        assert_eq!(format!("{}", ToolVerb::Stay), "Stay");
        assert_eq!(format!("{}", ToolVerb::Wander), "Wander");
        assert_eq!(format!("{}", ToolVerb::Hide), "Hide");
        assert_eq!(format!("{}", ToolVerb::Rally), "Rally");
    }

    #[test]
    fn test_tool_verb_is_movement() {
        assert!(ToolVerb::MoveTo.is_movement());
        assert!(ToolVerb::Wander.is_movement());
        assert!(ToolVerb::Hide.is_movement());
        assert!(ToolVerb::Stay.is_movement());
        assert!(!ToolVerb::Throw.is_movement());
        assert!(!ToolVerb::CoverFire.is_movement());
        assert!(!ToolVerb::Revive.is_movement());
    }

    #[test]
    fn test_tool_verb_is_combat() {
        assert!(ToolVerb::Throw.is_combat());
        assert!(ToolVerb::CoverFire.is_combat());
        assert!(!ToolVerb::MoveTo.is_combat());
        assert!(!ToolVerb::Revive.is_combat());
        assert!(!ToolVerb::Rally.is_combat());
    }

    #[test]
    fn test_tool_verb_is_support() {
        assert!(ToolVerb::Revive.is_support());
        assert!(ToolVerb::Rally.is_support());
        assert!(!ToolVerb::MoveTo.is_support());
        assert!(!ToolVerb::Throw.is_support());
        assert!(!ToolVerb::CoverFire.is_support());
    }

    #[test]
    fn test_tool_verb_requires_target_position() {
        assert!(ToolVerb::MoveTo.requires_target_position());
        assert!(ToolVerb::Throw.requires_target_position());
        assert!(ToolVerb::CoverFire.requires_target_position());
        assert!(ToolVerb::Revive.requires_target_position());
        assert!(ToolVerb::Hide.requires_target_position());
        assert!(!ToolVerb::Stay.requires_target_position());
        assert!(!ToolVerb::Wander.requires_target_position());
        assert!(!ToolVerb::Rally.requires_target_position());
    }

    #[test]
    fn test_tool_verb_requires_ammo() {
        assert!(ToolVerb::Throw.requires_ammo());
        assert!(ToolVerb::CoverFire.requires_ammo());
        assert!(!ToolVerb::MoveTo.requires_ammo());
        assert!(!ToolVerb::Revive.requires_ammo());
        assert!(!ToolVerb::Stay.requires_ammo());
    }

    #[test]
    fn test_tool_verb_requires_line_of_sight() {
        assert!(ToolVerb::Throw.requires_line_of_sight());
        assert!(ToolVerb::CoverFire.requires_line_of_sight());
        assert!(!ToolVerb::MoveTo.requires_line_of_sight());
        assert!(!ToolVerb::Revive.requires_line_of_sight());
        assert!(!ToolVerb::Stay.requires_line_of_sight());
    }

    #[test]
    fn test_tool_verb_primary_validation_category() {
        assert_eq!(
            ToolVerb::MoveTo.primary_validation_category(),
            ValidationCategory::Nav
        );
        assert_eq!(
            ToolVerb::Wander.primary_validation_category(),
            ValidationCategory::Nav
        );
        assert_eq!(
            ToolVerb::Hide.primary_validation_category(),
            ValidationCategory::Nav
        );
        assert_eq!(
            ToolVerb::Throw.primary_validation_category(),
            ValidationCategory::Visibility
        );
        assert_eq!(
            ToolVerb::CoverFire.primary_validation_category(),
            ValidationCategory::Visibility
        );
        assert_eq!(
            ToolVerb::Revive.primary_validation_category(),
            ValidationCategory::Resources
        );
        assert_eq!(
            ToolVerb::UseItem.primary_validation_category(),
            ValidationCategory::Resources
        );
        assert_eq!(
            ToolVerb::Interact.primary_validation_category(),
            ValidationCategory::Physics
        );
        assert_eq!(
            ToolVerb::Stay.primary_validation_category(),
            ValidationCategory::Cooldown
        );
        assert_eq!(
            ToolVerb::Rally.primary_validation_category(),
            ValidationCategory::Cooldown
        );
    }

    #[test]
    fn test_tool_verb_all() {
        let all = ToolVerb::all();
        assert_eq!(all.len(), 10);
        assert!(all.contains(&ToolVerb::MoveTo));
        assert!(all.contains(&ToolVerb::Throw));
        assert!(all.contains(&ToolVerb::CoverFire));
        assert!(all.contains(&ToolVerb::Rally));
    }

    // ========================================
    // ValidationCategory Display and Helper Tests
    // ========================================

    #[test]
    fn test_validation_category_display() {
        assert_eq!(format!("{}", ValidationCategory::Nav), "Nav");
        assert_eq!(format!("{}", ValidationCategory::Physics), "Physics");
        assert_eq!(format!("{}", ValidationCategory::Resources), "Resources");
        assert_eq!(format!("{}", ValidationCategory::Visibility), "Visibility");
        assert_eq!(format!("{}", ValidationCategory::Cooldown), "Cooldown");
    }

    #[test]
    fn test_validation_category_all() {
        let all = ValidationCategory::all();
        assert_eq!(all.len(), 5);
        assert!(all.contains(&ValidationCategory::Nav));
        assert!(all.contains(&ValidationCategory::Physics));
        assert!(all.contains(&ValidationCategory::Resources));
        assert!(all.contains(&ValidationCategory::Visibility));
        assert!(all.contains(&ValidationCategory::Cooldown));
    }

    #[test]
    fn test_validation_category_requires_external_system() {
        assert!(ValidationCategory::Nav.requires_external_system());
        assert!(ValidationCategory::Physics.requires_external_system());
        assert!(!ValidationCategory::Resources.requires_external_system());
        assert!(!ValidationCategory::Visibility.requires_external_system());
        assert!(!ValidationCategory::Cooldown.requires_external_system());
    }

    // ========================================
    // ToolError Helper Tests
    // ========================================

    #[test]
    fn test_tool_error_is_nav_error() {
        assert!(ToolError::OutOfBounds.is_nav_error());
        assert!(ToolError::NoPath.is_nav_error());
        assert!(!ToolError::Cooldown.is_nav_error());
        assert!(!ToolError::PhysicsBlocked.is_nav_error());
    }

    #[test]
    fn test_tool_error_is_physics_error() {
        assert!(ToolError::PhysicsBlocked.is_physics_error());
        assert!(!ToolError::OutOfBounds.is_physics_error());
        assert!(!ToolError::Cooldown.is_physics_error());
    }

    #[test]
    fn test_tool_error_is_resource_error() {
        assert!(ToolError::InsufficientResource.is_resource_error());
        assert!(!ToolError::OutOfBounds.is_resource_error());
        assert!(!ToolError::Cooldown.is_resource_error());
    }

    #[test]
    fn test_tool_error_is_visibility_error() {
        assert!(ToolError::NoLineOfSight.is_visibility_error());
        assert!(!ToolError::OutOfBounds.is_visibility_error());
        assert!(!ToolError::Cooldown.is_visibility_error());
    }

    #[test]
    fn test_tool_error_is_target_error() {
        assert!(ToolError::InvalidTarget.is_target_error());
        assert!(!ToolError::OutOfBounds.is_target_error());
        assert!(!ToolError::Cooldown.is_target_error());
    }

    #[test]
    fn test_tool_error_is_timing_error() {
        assert!(ToolError::Cooldown.is_timing_error());
        assert!(!ToolError::OutOfBounds.is_timing_error());
        assert!(!ToolError::NoPath.is_timing_error());
    }

    #[test]
    fn test_tool_error_is_recoverable() {
        assert!(ToolError::Cooldown.is_recoverable());
        assert!(ToolError::NoLineOfSight.is_recoverable());
        assert!(!ToolError::OutOfBounds.is_recoverable());
        assert!(!ToolError::NoPath.is_recoverable());
        assert!(!ToolError::PhysicsBlocked.is_recoverable());
    }

    #[test]
    fn test_tool_error_category() {
        assert_eq!(ToolError::OutOfBounds.category(), ValidationCategory::Nav);
        assert_eq!(ToolError::NoPath.category(), ValidationCategory::Nav);
        assert_eq!(
            ToolError::PhysicsBlocked.category(),
            ValidationCategory::Physics
        );
        assert_eq!(
            ToolError::InsufficientResource.category(),
            ValidationCategory::Resources
        );
        assert_eq!(
            ToolError::NoLineOfSight.category(),
            ValidationCategory::Visibility
        );
        assert_eq!(ToolError::Cooldown.category(), ValidationCategory::Cooldown);
        assert_eq!(
            ToolError::InvalidTarget.category(),
            ValidationCategory::Resources
        );
        assert_eq!(ToolError::Unknown.category(), ValidationCategory::Resources);
    }

    #[test]
    fn test_tool_error_all() {
        let all = ToolError::all();
        assert_eq!(all.len(), 8);
        assert!(all.contains(&ToolError::OutOfBounds));
        assert!(all.contains(&ToolError::Cooldown));
        assert!(all.contains(&ToolError::NoLineOfSight));
        assert!(all.contains(&ToolError::Unknown));
    }

    #[test]
    fn test_tool_error_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(ToolError::OutOfBounds);
        set.insert(ToolError::Cooldown);
        set.insert(ToolError::NoPath);
        assert_eq!(set.len(), 3);
    }

    // ========================================
    // ValidationContext Helper Tests
    // ========================================

    #[test]
    fn test_validation_context_has_nav() {
        let context_empty = ValidationContext::new();
        assert!(!context_empty.has_nav());

        // Test that with_nav sets has_nav to true
        let nav = NavMesh::bake(&[], 0.4, 60.0);
        let context_with_nav = ValidationContext::new().with_nav(&nav);
        assert!(
            context_with_nav.has_nav(),
            "with_nav should make has_nav() return true"
        );
    }

    #[test]
    fn test_validation_context_has_physics() {
        let context_empty = ValidationContext::new();
        assert!(!context_empty.has_physics());

        // Test with physics
        let rigid_body_set = RigidBodySet::new();
        let collider_set = ColliderSet::new();
        let physics_pipeline = PhysicsPipeline::new();
        let context_with_physics = ValidationContext::new().with_physics(
            &physics_pipeline,
            &rigid_body_set,
            &collider_set,
        );
        assert!(context_with_physics.has_physics());
    }

    #[test]
    fn test_validation_context_is_complete() {
        let context_empty = ValidationContext::new();
        assert!(!context_empty.is_complete());

        // Context with only physics is not complete
        let rigid_body_set = RigidBodySet::new();
        let collider_set = ColliderSet::new();
        let physics_pipeline = PhysicsPipeline::new();
        let context_physics_only = ValidationContext::new().with_physics(
            &physics_pipeline,
            &rigid_body_set,
            &collider_set,
        );
        assert!(!context_physics_only.is_complete());

        // Context with only nav is not complete
        let nav = NavMesh::bake(&[], 0.4, 60.0);
        let context_nav_only = ValidationContext::new().with_nav(&nav);
        assert!(
            !context_nav_only.is_complete(),
            "Nav-only should not be complete"
        );

        // Context with both nav AND physics should be complete
        let context_full = ValidationContext::new().with_nav(&nav).with_physics(
            &physics_pipeline,
            &rigid_body_set,
            &collider_set,
        );
        assert!(
            context_full.is_complete(),
            "With both nav and physics, is_complete should be true"
        );
    }

    #[test]
    fn test_validation_context_is_empty() {
        let context_empty = ValidationContext::new();
        assert!(context_empty.is_empty());

        let rigid_body_set = RigidBodySet::new();
        let collider_set = ColliderSet::new();
        let physics_pipeline = PhysicsPipeline::new();
        let context_with_physics = ValidationContext::new().with_physics(
            &physics_pipeline,
            &rigid_body_set,
            &collider_set,
        );
        assert!(!context_with_physics.is_empty());
    }

    #[test]
    fn test_validation_context_available_categories() {
        let context_empty = ValidationContext::new();
        let cats_empty = context_empty.available_categories();
        // Should have Resources, Visibility, Cooldown
        assert!(cats_empty.contains(&ValidationCategory::Resources));
        assert!(cats_empty.contains(&ValidationCategory::Visibility));
        assert!(cats_empty.contains(&ValidationCategory::Cooldown));
        assert!(!cats_empty.contains(&ValidationCategory::Nav));
        assert!(!cats_empty.contains(&ValidationCategory::Physics));

        // With physics
        let rigid_body_set = RigidBodySet::new();
        let collider_set = ColliderSet::new();
        let physics_pipeline = PhysicsPipeline::new();
        let context_with_physics = ValidationContext::new().with_physics(
            &physics_pipeline,
            &rigid_body_set,
            &collider_set,
        );
        let cats_physics = context_with_physics.available_categories();
        assert!(cats_physics.contains(&ValidationCategory::Physics));
        assert!(!cats_physics.contains(&ValidationCategory::Nav));
    }

    // ========================================
    // Mutation-Killing Tests (tool_sandbox.rs)
    // Catches: has_physics &&→||, cooldown >=→>,
    //          morale <→<=, revive dist *→+,
    //          LOS obs &&→||, LOS from non-origin
    // ========================================

    #[test]
    fn test_has_physics_partial_only_pipeline() {
        // has_physics uses `&&`: ALL three must be Some.
        // Setting only physics_pipeline but not rigid_body/collider → false.
        // Mutation `&& → ||` would make this true incorrectly.
        let pipeline = PhysicsPipeline::new();
        let ctx = ValidationContext {
            nav_mesh: None,
            physics_pipeline: Some(&pipeline),
            rigid_body_set: None,
            collider_set: None,
        };
        assert!(
            !ctx.has_physics(),
            "has_physics should be false when only pipeline is set (&&, not ||)"
        );
    }

    #[test]
    fn test_has_physics_partial_missing_collider() {
        // physics_pipeline + rigid_body_set but NO collider_set → false.
        // Catches the second `&&` mutation.
        let pipeline = PhysicsPipeline::new();
        let bodies = RigidBodySet::new();
        let ctx = ValidationContext {
            nav_mesh: None,
            physics_pipeline: Some(&pipeline),
            rigid_body_set: Some(&bodies),
            collider_set: None,
        };
        assert!(
            !ctx.has_physics(),
            "has_physics should be false when collider_set is missing"
        );
    }

    #[test]
    fn test_cooldown_zero_allows_action() {
        // Cooldown = 0.0 should NOT block (> 0.0 is false).
        // Mutation `> → >=` would block at 0.0.
        let mut cooldowns = std::collections::BTreeMap::new();
        cooldowns.insert("throw".to_string(), 0.0);
        let world = WorldSnapshot {
            t: 0.0,
            player: PlayerState {
                hp: 100,
                pos: IVec2 { x: 0, y: 0 },
                stance: "standing".into(),
                orders: vec![],
            },
            me: CompanionState {
                ammo: 10,
                cooldowns,
                morale: 1.0,
                pos: IVec2 { x: 0, y: 0 },
            },
            enemies: vec![],
            pois: vec![],
            objective: None,
            obstacles: vec![],
        };
        let context = ValidationContext::new();
        let result = validate_tool_action(
            0,
            ToolVerb::Throw,
            &world,
            &context,
            Some(IVec2 { x: 1, y: 0 }),
        );
        assert!(
            result.is_ok(),
            "Cooldown=0.0 should not block (> 0.0 = false)"
        );
    }

    #[test]
    fn test_revive_morale_exact_boundary() {
        // Morale = 0.5: `< 0.5` is false → allowed.
        // Mutation `< → <=` would block at exactly 0.5.
        let world = WorldSnapshot {
            t: 0.0,
            player: PlayerState {
                hp: 100,
                pos: IVec2 { x: 0, y: 0 },
                stance: "standing".into(),
                orders: vec![],
            },
            me: CompanionState {
                ammo: 10,
                cooldowns: std::collections::BTreeMap::new(),
                morale: 0.5, // Exactly at boundary
                pos: IVec2 { x: 0, y: 0 },
            },
            enemies: vec![],
            pois: vec![],
            objective: None,
            obstacles: vec![],
        };
        let context = ValidationContext::new();
        let result = validate_tool_action(
            0,
            ToolVerb::Revive,
            &world,
            &context,
            Some(IVec2 { x: 1, y: 0 }),
        );
        assert!(
            result.is_ok(),
            "Revive at morale=0.5 should pass (< 0.5 is false)"
        );
    }

    #[test]
    fn test_revive_distance_negative_dx_catches_multiply_vs_add() {
        // When dx is negative: dx*dx = positive, but dx+dx = negative.
        // (dx+dx + dy*dy) could be negative → sqrt(NaN) → NaN > 2.0 = false → OK.
        // We need a case where *→+ changes the outcome.
        // me at (5,0), target at (2,0): dx = 2-5 = -3.
        // Original: (-3)*(-3) = 9, sqrt(9) = 3.0 > 2.0 → FAIL.
        // Mutation: (-3)+(-3) = -6, (-6+0) as f32 → sqrt(-6) = NaN, NaN > 2.0 = false → OK.
        let world = WorldSnapshot {
            t: 0.0,
            player: PlayerState {
                hp: 100,
                pos: IVec2 { x: 0, y: 0 },
                stance: "standing".into(),
                orders: vec![],
            },
            me: CompanionState {
                ammo: 10,
                cooldowns: std::collections::BTreeMap::new(),
                morale: 1.0,
                pos: IVec2 { x: 5, y: 0 },
            },
            enemies: vec![],
            pois: vec![],
            objective: None,
            obstacles: vec![],
        };
        let context = ValidationContext::new();
        let result = validate_tool_action(
            0,
            ToolVerb::Revive,
            &world,
            &context,
            Some(IVec2 { x: 2, y: 0 }),
        );
        assert!(
            result.is_err(),
            "Revive at distance 3.0 should fail (target too far)"
        );
    }

    #[test]
    fn test_revive_distance_negative_dy_catches_multiply_vs_add() {
        // Same idea for dy mutation: me at (0,4), target at (0,1).
        // dy = 1-4 = -3. Original: (-3)² = 9. Mutation: -3+(-3) = -6.
        // sqrt(0+9) = 3 > 2 → fail. Mutation: sqrt(0+(-6)) = NaN → OK.
        let world = WorldSnapshot {
            t: 0.0,
            player: PlayerState {
                hp: 100,
                pos: IVec2 { x: 0, y: 0 },
                stance: "standing".into(),
                orders: vec![],
            },
            me: CompanionState {
                ammo: 10,
                cooldowns: std::collections::BTreeMap::new(),
                morale: 1.0,
                pos: IVec2 { x: 0, y: 4 },
            },
            enemies: vec![],
            pois: vec![],
            objective: None,
            obstacles: vec![],
        };
        let context = ValidationContext::new();
        let result = validate_tool_action(
            0,
            ToolVerb::Revive,
            &world,
            &context,
            Some(IVec2 { x: 0, y: 1 }),
        );
        assert!(
            result.is_err(),
            "Revive at distance 3.0 should fail (dy*dy vs dy+dy)"
        );
    }

    #[test]
    fn test_los_obstacle_single_coordinate_match() {
        // Obstacle at (5,1) shares y=1 with path cell (0,1) on vertical line.
        // Original: obs.x == x && obs.y == y → 5==0 && 1==1 → false → no block.
        // Mutation `&& → ||`: 5==0 || 1==1 → true → blocked incorrectly.
        let world = WorldSnapshot {
            t: 0.0,
            player: PlayerState {
                hp: 100,
                pos: IVec2 { x: 0, y: 0 },
                stance: "standing".into(),
                orders: vec![],
            },
            me: CompanionState {
                ammo: 10,
                cooldowns: std::collections::BTreeMap::new(),
                morale: 1.0,
                pos: IVec2 { x: 0, y: 0 },
            },
            enemies: vec![],
            pois: vec![],
            objective: None,
            obstacles: vec![IVec2 { x: 5, y: 1 }], // Shares y=1 with path cell (0,1)
        };
        // LOS from (0,0) to (0,3) — vertical line, obstacle at (5,1) NOT on path.
        let has_los = has_line_of_sight(IVec2 { x: 0, y: 0 }, IVec2 { x: 0, y: 3 }, &world);
        assert!(
            has_los,
            "Obstacle at (5,1) should NOT block vertical LOS from (0,0) to (0,3)"
        );
    }

    #[test]
    fn test_los_from_nonzero_origin_with_obstacle() {
        // Uses non-zero from to catch `(to.x - from.x).abs() → (to.x + from.x).abs()`
        // from (3,2) to (6,4): original dx=3, dy=2.
        // Mutation: dx=abs(6+3)=9, dy=abs(4+2)=6 — different Bresenham path.
        // With the mutation, Bresenham may diverge (timeout=caught) or miss obstacle.
        let world = WorldSnapshot {
            t: 0.0,
            player: PlayerState {
                hp: 100,
                pos: IVec2 { x: 0, y: 0 },
                stance: "standing".into(),
                orders: vec![],
            },
            me: CompanionState {
                ammo: 10,
                cooldowns: std::collections::BTreeMap::new(),
                morale: 1.0,
                pos: IVec2 { x: 3, y: 2 },
            },
            enemies: vec![],
            pois: vec![],
            objective: None,
            obstacles: vec![IVec2 { x: 5, y: 3 }], // On original path (3,2)→(4,3)→(5,3)→(6,4)
        };
        let has_los = has_line_of_sight(IVec2 { x: 3, y: 2 }, IVec2 { x: 6, y: 4 }, &world);
        assert!(
            !has_los,
            "Obstacle at (5,3) should block LOS from (3,2) to (6,4)"
        );
    }
}
