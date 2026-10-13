//! Headless greybox simulation. The window binary only displays this.

use astraweave_camera::ThirdPersonCamera;
use astraweave_nav::{Aabb, NavMesh};
use astraweave_physics::PhysicsWorld;
use astraweave_terrain::VoxelGrid;
use glam::Vec3;

use crate::combat::Combat;
use crate::ground::{ChunkUpload, Ground};
use crate::locomotion::{clip_name, visual_time, LocomotionGraph, PoseInput};
use crate::motor::{Motor, MotorInput};
use crate::player::{self, Player};
pub const FIXED_DT: f32 = 1.0 / 60.0;

fn frame_camera(pivot: Vec3) -> ThirdPersonCamera {
    let mut camera = ThirdPersonCamera::new(pivot);
    camera.distance = 4.2;
    camera.pitch = -0.25;
    camera
}
/// Chebyshev chunk radius. 3 keeps about 224 m of terrain loaded, which stays
/// comfortable on a GTX 1660 with these low-poly chunks and props.
pub const VIEW_RADIUS: i32 = 3;

pub struct Greybox {
    pub physics: PhysicsWorld,
    pub ground: Ground,
    pub player: Player,
    pub camera: ThirdPersonCamera,
    pub motor: Motor,
    pub graph: LocomotionGraph,
    pub combat: Combat,
    pub nav: NavMesh,
    pub pending_uploads: Vec<ChunkUpload>,
    /// Meshes from the first sync, moved out by the window once a device exists.
    pub initial_uploads: Vec<ChunkUpload>,
    /// Negative skips streaming so a hand-built island is left alone.
    view_radius: i32,
    spawn: Vec3,
}

impl Greybox {
    pub fn new(grid: VoxelGrid, spawn: Vec3) -> Self {
        let mut physics = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));
        let mut ground = Ground::from_grid(grid);
        let initial_uploads = ground.sync_dirty(&mut physics);
        // Query pipeline only sees the new trimeshes after a step.
        physics.step();
        let player = Player::spawn(&mut physics, spawn);
        let camera = frame_camera(spawn);
        Self {
            physics,
            ground,
            player,
            camera,
            motor: Motor::default(),
            graph: LocomotionGraph::default(),
            combat: Combat::default(),
            nav: NavMesh::bake(&[], 0.5, 55.0),
            pending_uploads: Vec::new(),
            initial_uploads,
            view_radius: -1,
            spawn,
        }
    }

    pub fn open(radius: i32) -> Self {
        let spawn = Vec3::new(16.0, 20.0, 16.0);
        let mut physics = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));
        let mut ground = Ground::from_grid(VoxelGrid::new());
        let initial_uploads = ground.stream_around(&mut physics, spawn, radius);
        physics.step();
        let player = Player::spawn(&mut physics, spawn);
        let camera = frame_camera(spawn);
        Self {
            physics,
            ground,
            player,
            camera,
            motor: Motor::default(),
            graph: LocomotionGraph::default(),
            combat: Combat::default(),
            nav: NavMesh::bake(&[], 0.5, 55.0),
            pending_uploads: Vec::new(),
            initial_uploads,
            view_radius: radius,
            spawn,
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
        let mut input = input;
        if self.combat.player_hp <= 0.0 {
            input = MotorInput::default();
        }
        input.feet_locked = self.combat.attack.locks_feet();
        self.motor.tick(
            &mut self.physics,
            &mut self.player,
            self.camera.yaw,
            input,
            FIXED_DT,
        );
        self.physics.step();
        self.combat
            .tick(&mut self.physics, &self.player, &self.motor, FIXED_DT);
        if let Some(index) = self.combat.lock {
            if let Some(enemy) = self.combat.enemies.get(index) {
                if let Some(pos) = crate::combat::body_pos(&self.physics, enemy.body) {
                    let to = pos - self.player_pos();
                    self.camera.yaw = to.z.atan2(to.x);
                }
            }
        }
        let moving = input.move_x.abs() + input.move_y.abs() > 0.05;
        self.graph.sync(
            PoseInput {
                mode: self.motor.mode,
                grounded: self.motor.grounded,
                crouching: self.motor.crouching,
                sprinting: self.motor.sprinting,
                walking: self.motor.walking,
                moving,
                attacking: self.combat.attack.swinging,
                blocking: self.combat.blocking,
                hit: self.combat.hit_react > 0.0,
                dead: self.combat.player_hp <= 0.0,
            },
            FIXED_DT,
        );
        self.camera.pivot = self.player_pos();
        if self.view_radius >= 0 {
            let focus = self.player_pos();
            let streamed = self
                .ground
                .stream_around(&mut self.physics, focus, self.view_radius);
            self.pending_uploads.extend(streamed);
        }
    }

    pub fn checkpoint(&self) -> crate::save::Checkpoint {
        crate::save::Checkpoint {
            edits: self.ground.edits.clone(),
            position: self.player_pos(),
            yaw: self.player.facing_yaw,
            stamina: self.motor.stamina,
            player_hp: self.combat.player_hp,
        }
    }

    pub fn restore(&mut self, checkpoint: crate::save::Checkpoint) -> Vec<ChunkUpload> {
        self.motor.stamina = checkpoint.stamina.clamp(0.0, self.motor.stamina_max);
        self.combat.player_hp = checkpoint.player_hp.max(0.0);
        self.player.facing_yaw = checkpoint.yaw;
        self.physics
            .set_body_position(self.player.body, checkpoint.position);
        self.camera.pivot = checkpoint.position;
        self.ground.restore_edits(checkpoint.edits);
        let uploads = self.ground.sync_dirty(&mut self.physics);
        self.physics.step();
        uploads
    }

    /// Full health at the original spawn. The edit log stays.
    pub fn respawn(&mut self) {
        self.combat.player_hp = 100.0;
        self.combat.hit_react = 0.0;
        self.combat.attack = crate::combat::Attack::default();
        self.motor = Motor::default();
        self.graph = LocomotionGraph::default();
        if let Some(ctrl) = self.physics.char_map.get_mut(&self.player.body) {
            ctrl.gravity_scale = 1.0;
            ctrl.vertical_velocity = 0.0;
        }
        self.physics.set_body_position(self.player.body, self.spawn);
        self.camera.pivot = self.spawn;
    }

    pub fn spawn_enemy(&mut self, pos: Vec3) {
        let body = self.physics.add_character(pos, Vec3::new(0.35, 0.8, 0.35));
        self.combat.enemies.push(crate::combat::Enemy::new(body));
    }
    pub fn dig_ahead(&mut self, radius: f32) -> Vec<ChunkUpload> {
        let pos = self.player_pos();
        let (forward, _) = player::flat_basis(self.player.facing_yaw);
        let center = pos + forward * 1.5 + Vec3::new(0.0, -1.2, 0.0);
        self.ground.dig_sphere(center, radius);
        self.nav
            .invalidate_region(Aabb::from_center_half_extents(center, Vec3::splat(radius)));
        let uploads = self.ground.sync_dirty(&mut self.physics);
        self.physics.step();
        uploads
    }

    pub fn place_ahead(&mut self, radius: f32, material: u16) -> Vec<ChunkUpload> {
        let pos = self.player_pos();
        let (forward, _) = player::flat_basis(self.player.facing_yaw);
        let center = pos + forward * 1.8;
        self.ground.place_sphere(center, radius, material);
        self.nav
            .invalidate_region(Aabb::from_center_half_extents(center, Vec3::splat(radius)));
        let uploads = self.ground.sync_dirty(&mut self.physics);
        self.physics.step();
        uploads
    }

    /// Eye distance after a ray from the pivot toward the camera eye.
    /// Clip name and the sample clock for the player mesh.
    pub fn player_visual(&self, clip_duration: f32) -> (&'static str, f32, bool) {
        let state = self.graph.state;
        let name = clip_name(state);
        let duration = clip_duration;
        let phase = match state {
            crate::locomotion::LocoState::Roll => {
                self.motor.roll_time / crate::motor::ROLL_DURATION
            }
            crate::locomotion::LocoState::Attack => {
                self.combat.attack.time / crate::combat::ATTACK_WINDOW
            }
            crate::locomotion::LocoState::Hit => {
                1.0 - (self.combat.hit_react / crate::combat::HIT_REACT).clamp(0.0, 1.0)
            }
            _ => 0.0,
        };
        let (time, looping) = visual_time(state, self.graph.time, phase, duration);
        (name, time, looping)
    }

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

    #[test]
    fn attacking_selects_the_sword_clip() {
        let mut sim = Greybox::island();
        sim.combat.attack.start();
        sim.tick_controls(MotorInput::default());
        let (name, time, looping) = sim.player_visual(1.5);
        assert_eq!(name, "Sword_Attack");
        assert!(!looping);
        assert!(time >= 0.0);
    }

    #[test]
    fn save_and_load_round_trip_keeps_the_hole_and_the_body() {
        let mut sim = Greybox::open(0);
        let feet = sim.player_pos();
        sim.ground.dig_sphere(Vec3::new(feet.x, 4.0, feet.z), 1.2);
        let _ = sim.ground.sync_dirty(&mut sim.physics);
        sim.combat.player_hp = 41.0;
        sim.motor.stamina = 18.0;
        sim.player.facing_yaw = 0.7;
        let dir = tempfile::tempdir().unwrap();
        crate::save::save_checkpoint(dir.path(), &sim.checkpoint()).unwrap();
        sim.ground.dig_sphere(Vec3::new(20.0, 4.0, 16.0), 1.0);
        sim.combat.player_hp = 0.0;
        let loaded = crate::save::load_checkpoint(dir.path()).unwrap();
        let _ = sim.restore(loaded);
        assert!((sim.combat.player_hp - 41.0).abs() < 1e-3);
        assert!((sim.motor.stamina - 18.0).abs() < 1e-3);
        assert!((sim.player.facing_yaw - 0.7).abs() < 1e-3);
        let hole = sim.ground.grid.get_voxel(Vec3::new(feet.x, 4.0, feet.z));
        assert!(hole.is_some_and(|voxel| !voxel.is_solid()));
        let later = sim.ground.grid.get_voxel(Vec3::new(20.0, 4.0, 16.0));
        assert!(
            later.is_some_and(|voxel| voxel.is_solid()),
            "a dig made after the save must not survive the load"
        );
        crate::save::save_checkpoint(dir.path(), &sim.checkpoint()).unwrap();
        let again = crate::save::load_checkpoint(dir.path()).unwrap();
        assert!((again.player_hp - 41.0).abs() < 1e-3);
        let missing = dir.path().join("missing");
        assert!(crate::save::load_checkpoint(&missing).is_err());
    }

    #[test]
    fn digging_under_a_streamed_player_stays_finite() {
        let mut sim = Greybox::open(0);
        for _ in 0..40 {
            sim.tick_controls(MotorInput::default());
        }
        let pos = sim.player_pos();
        assert!(pos.is_finite(), "{pos:?}");
        sim.ground
            .dig_sphere(Vec3::new(pos.x, pos.y - 1.2, pos.z), 5.0);
        let _ = sim.ground.sync_dirty(&mut sim.physics);
        sim.physics.step();
        for _ in 0..40 {
            sim.tick_controls(MotorInput::default());
            sim.ground.dig_sphere(
                Vec3::new(
                    sim.player_pos().x,
                    sim.player_pos().y - 2.0,
                    sim.player_pos().z,
                ),
                1.5,
            );
            let _ = sim.ground.sync_dirty(&mut sim.physics);
        }
        let fallen = sim.player_pos();
        assert!(fallen.is_finite(), "{fallen:?}");
        assert!(pos.y - fallen.y > 0.5, "{pos:?} -> {fallen:?}");
    }

    #[test]
    fn death_respawn_returns_to_the_spawn() {
        let mut sim = Greybox::open(0);
        let spawn = sim.player_pos();
        sim.combat.player_hp = 0.0;
        sim.motor.mode = crate::motor::MotorMode::Fly;
        sim.physics
            .set_body_position(sim.player.body, Vec3::new(8.0, -40.0, 8.0));
        sim.respawn();
        assert!((sim.combat.player_hp - 100.0).abs() < 1e-3);
        assert_eq!(sim.motor.mode, crate::motor::MotorMode::Locomotion);
        let back = sim.player_pos();
        assert!(
            (back - spawn).length() < 0.2,
            "respawn {back:?} spawn {spawn:?}"
        );
        sim.tick_controls(MotorInput::default());
        let (clip, _, _) = sim.player_visual(2.0);
        assert_ne!(clip, "Death01");
    }

    #[test]
    fn flying_across_a_chunk_edge_stays_airborne() {
        let mut sim = Greybox::open(1);
        let start = sim.player_pos();
        sim.tick_controls(MotorInput {
            fly_toggle: true,
            ..MotorInput::default()
        });
        for _ in 0..180 {
            sim.tick_controls(MotorInput {
                move_y: 1.0,
                ..MotorInput::default()
            });
        }
        let end = sim.player_pos();
        assert!(end.is_finite(), "{end:?}");
        assert!(end.x > start.x + 16.0, "{start:?} -> {end:?}");
        assert!(
            (end.y - start.y).abs() < 3.0,
            "fly should not fall through a chunk seam {start:?} -> {end:?}"
        );
    }
}
