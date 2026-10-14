//! Kinematic capsule driven by [`PhysicsWorld::control_character`].

use astraweave_physics::PhysicsWorld;
use glam::Vec3;

pub struct Player {
    pub body: u64,
    pub facing_yaw: f32,
}

impl Player {
    pub fn spawn(physics: &mut PhysicsWorld, pos: Vec3) -> Self {
        let body = physics.add_character(pos, Vec3::new(0.35, 0.8, 0.35));
        Self {
            body,
            facing_yaw: 0.0,
        }
    }

    pub fn translation(&self, physics: &PhysicsWorld) -> Option<Vec3> {
        physics
            .body_transform(self.body)
            .map(|m| m.w_axis.truncate())
    }

    pub fn is_grounded(&self, physics: &PhysicsWorld) -> bool {
        physics
            .char_map
            .get(&self.body)
            .is_some_and(|c| c.is_grounded())
    }

    /// `wish` is a world-space horizontal velocity in meters per second.
    pub fn tick(&mut self, physics: &mut PhysicsWorld, wish: Vec3, dt: f32) {
        physics.control_character(self.body, wish, dt, false);
        physics.step();
    }
}

/// Camera-relative flat wish from key axes. `forward`/`right` are unit XZ vectors.
pub fn wish_from_axes(forward: Vec3, right: Vec3, x: f32, y: f32, speed: f32) -> Vec3 {
    let mut wish = forward * y + right * x;
    if wish.length_squared() > 1e-6 {
        wish = wish.normalize() * speed;
    }
    Vec3::new(wish.x, 0.0, wish.z)
}

pub fn flat_basis(yaw: f32) -> (Vec3, Vec3) {
    let forward = Vec3::new(crate::detmath::cos(yaw), 0.0, crate::detmath::sin(yaw));
    let right = Vec3::new(-crate::detmath::sin(yaw), 0.0, crate::detmath::cos(yaw));
    (forward, right)
}
