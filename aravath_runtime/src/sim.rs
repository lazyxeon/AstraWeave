//! Headless greybox simulation. The window binary only displays this.

use astraweave_camera::ThirdPersonCamera;
use astraweave_physics::PhysicsWorld;
use astraweave_terrain::VoxelGrid;
use glam::Vec3;

use crate::ground::{ChunkUpload, Ground};
use crate::motor::{Motor, MotorInput};
use crate::player::{self, Player};
pub const FIXED_DT: f32 = 1.0 / 60.0;

pub struct Greybox {
    pub physics: PhysicsWorld,
    pub ground: Ground,
    pub player: Player,
    pub camera: ThirdPersonCamera,
    pub motor: Motor,
    /// Meshes from the first sync, moved out by the window once a device exists.
    pub initial_uploads: Vec<ChunkUpload>,
}

impl Greybox {
    pub fn new(grid: VoxelGrid, spawn: Vec3) -> Self {
        let mut physics = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));
        let mut ground = Ground::from_grid(grid);
        let initial_uploads = ground.sync_dirty(&mut physics);
        // Query pipeline only sees the new trimeshes after a step.
        physics.step();
        let player = Player::spawn(&mut physics, spawn);
        let camera = ThirdPersonCamera::new(spawn);
        Self {
            physics,
            ground,
            player,
            camera,
            motor: Motor::default(),
            initial_uploads,
        }
    }

    pub fn island() -> Self {
        let grid = crate::island::small();
        Self::new(grid, Vec3::new(24.0, 18.0, 24.0))
    }

    pub fn player_pos(&self) -> Vec3 {
        self.player.translation(&self.physics).unwrap_or(Vec3::ZERO)
    }

    pub fn tick_wish(&mut self, wish: Vec3) {
        if wish.length_squared() > 1e-6 {
            self.player.facing_yaw = wish.z.atan2(wish.x);
        }
        self.player.tick(&mut self.physics, wish, FIXED_DT);
        self.camera.pivot = self.player_pos();
    }

    pub fn tick_controls(&mut self, input: MotorInput) {
        self.motor.tick(
            &mut self.physics,
            &mut self.player,
            self.camera.yaw,
            input,
            FIXED_DT,
        );
        self.physics.step();
        self.camera.pivot = self.player_pos();
    }

    /// Dig just in front of the feet. Returns chunk uploads for the renderer.
    pub fn dig_ahead(&mut self, radius: f32) -> Vec<ChunkUpload> {
        let pos = self.player_pos();
        let (forward, _) = player::flat_basis(self.player.facing_yaw);
        let center = pos + forward * 1.5 + Vec3::new(0.0, -1.2, 0.0);
        self.ground.dig_sphere(center, radius);
        let uploads = self.ground.sync_dirty(&mut self.physics);
        self.physics.step();
        uploads
    }

    /// Eye distance after a ray from the pivot toward the camera eye.
    pub fn occluded_camera_distance(&self) -> f32 {
        let dir = self.camera.eye_offset_dir();
        let hit = self
            .physics
            .raycast(self.camera.pivot, dir, self.camera.distance)
            .map(|(_, _, _, toi)| toi);
        self.camera.distance_after_hit(hit)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ground::Ground;
    use crate::island;
    use astraweave_camera::ThirdPersonCamera;

    fn settle(sim: &mut Greybox, frames: u32) -> Vec3 {
        for _ in 0..frames {
            sim.tick_wish(Vec3::ZERO);
        }
        sim.player_pos()
    }

    #[test]
    fn character_stands_on_island() {
        let mut sim = Greybox::island();
        let landed = settle(&mut sim, 180);
        assert!(
            sim.player.is_grounded(&sim.physics),
            "expected grounded, y={}",
            landed.y
        );
        let y = landed.y;
        let later = settle(&mut sim, 30);
        assert!(
            (later.y - y).abs() < 0.05,
            "drifted from {y} to {}",
            later.y
        );
        assert!(later.y > 6.0, "fell through the island, y={}", later.y);
        assert!(later.y < 16.0, "never reached the surface, y={}", later.y);
    }

    #[test]
    fn dig_under_character_drops_them() {
        let mut sim = Greybox::island();
        let before = settle(&mut sim, 180);
        assert!(sim.player.is_grounded(&sim.physics));
        sim.ground
            .dig_sphere(Vec3::new(before.x, before.y - 1.5, before.z), 4.0);
        sim.ground.sync_dirty(&mut sim.physics);
        sim.physics.step();
        let after = settle(&mut sim, 90);
        assert!(
            before.y - after.y > 1.0,
            "expected a drop, before {before:?} after {after:?}"
        );
    }

    #[test]
    fn clean_sync_preserves_body_ids() {
        let mut physics = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));
        let mut ground = Ground::small_island();
        let first = ground.sync_dirty(&mut physics);
        assert!(!first.is_empty(), "island produced no chunks");
        let ids = ground.body_ids();
        assert!(!ids.is_empty());
        let second = ground.sync_dirty(&mut physics);
        assert!(second.is_empty(), "clean sync allocated uploads");
        assert_eq!(ids, ground.body_ids());
    }

    #[test]
    fn camera_ray_shortens_against_solid_chunk() {
        let mut physics = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));
        let mut ground = Ground::from_grid(island::solid_box([8, 0, 8], [16, 20, 16]));
        ground.sync_dirty(&mut physics);
        physics.step();

        let mut camera = ThirdPersonCamera::new(Vec3::new(24.0, 12.0, 12.0));
        camera.yaw = 0.0;
        camera.pitch = 0.0;
        camera.distance = 16.0;
        let dir = camera.eye_offset_dir();
        let hit = physics
            .raycast(camera.pivot, dir, camera.distance)
            .map(|(_, _, _, toi)| toi);
        assert!(hit.is_some(), "ray missed the wall, dir {dir:?}");
        let resolved = camera.distance_after_hit(hit);
        assert!(resolved < camera.distance, "resolved {resolved}");
        assert!(resolved > 0.4);
    }
}
