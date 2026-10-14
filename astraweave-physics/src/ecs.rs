use crate::{BodyId, PhysicsWorld};
use astraweave_ecs::{App, Plugin, SystemStage, World};
use astraweave_scene::Transform;
use glam::Vec3;

/// Component that links an entity to a physics body
#[derive(Clone, Copy, Debug)]
pub struct PhysicsBodyComponent(pub BodyId);

/// Physics plugin for ECS integration
pub struct PhysicsPlugin;

impl Plugin for PhysicsPlugin {
    fn build(&self, app: &mut App) {
        // Insert PhysicsWorld resource
        app.world
            .insert_resource(PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0)));

        // Register systems in physics stage
        app.add_system(SystemStage::PHYSICS, physics_step_system)
            .expect("registered schedule stage");
        app.add_system(SystemStage::PHYSICS, sync_physics_to_transform_system)
            .expect("registered schedule stage");
    }
}

/// System that steps the physics simulation forward
pub fn physics_step_system(world: &mut World) {
    if let Some(physics_world) = world.get_resource_mut::<PhysicsWorld>() {
        physics_world.step();
    }
}

/// System that synchronizes physics body positions/rotations to Transform components
pub fn sync_physics_to_transform_system(world: &mut World) {
    // Get physics world resource
    let physics_world = match world.get_resource::<PhysicsWorld>() {
        Some(pw) => pw,
        None => return,
    };

    // Collect entities with PhysicsBodyComponent
    let mut updates: Vec<(astraweave_ecs::Entity, Transform)> = Vec::new();

    // Query all entities with PhysicsBodyComponent
    let entities = world.entities_with::<PhysicsBodyComponent>();
    for entity in entities {
        if let Some(physics_body) = world.get::<PhysicsBodyComponent>(entity) {
            let body_id = physics_body.0;

            // Get the body handle from physics world
            if let Some(handle) = physics_world.handle_of(body_id) {
                if let Some(rb) = physics_world.bodies.get(handle) {
                    let pos = rb.position();

                    // Convert physics position/rotation to Transform
                    let translation =
                        Vec3::new(pos.translation.x, pos.translation.y, pos.translation.z);
                    let rotation = glam::Quat::from_xyzw(
                        pos.rotation.i,
                        pos.rotation.j,
                        pos.rotation.k,
                        pos.rotation.w,
                    );

                    // Get current transform or create default
                    let current_transform =
                        world.get::<Transform>(entity).copied().unwrap_or_default();

                    let new_transform = Transform {
                        translation,
                        rotation,
                        scale: current_transform.scale, // Preserve scale
                    };

                    updates.push((entity, new_transform));
                }
            }
        }
    }

    // Apply updates
    for (entity, transform) in updates {
        world.insert(entity, transform);
    }
}
