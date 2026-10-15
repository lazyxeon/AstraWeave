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
use astraweave_veil::ScarField;
pub const FIXED_DT: f32 = 1.0 / 60.0;
/// Wall time longer than this is not fed into the accumulator.
pub const MAX_FRAME_DT: f32 = 0.25;
/// Physics slices spent before the rest of a stall is dropped.
pub const MAX_SUBSTEPS: u32 = 8;
const LOOK_SENSITIVITY: f32 = 0.005;

/// One fixed tick of player intent. The window samples this; it does not
/// pass a frame delta into movement, stamina, or combat.
#[derive(Clone, Copy, Debug, Default)]
pub struct TickInput {
    pub motor: MotorInput,
    pub attack: bool,
    pub block: bool,
    pub lock_on: bool,
    pub dig: bool,
    pub place: bool,
    pub place_material: u16,
    pub look_dx: f32,
    pub look_dy: f32,
    pub respawn: bool,
}

/// Edges sampled on a frame that did not spend a tick. The next tick takes them.
#[derive(Clone, Copy, Debug, Default)]
pub struct FrameLatch {
    pending: TickInput,
}

/// How many fixed ticks a frame spent, and whether one of them respawned.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CatchUp {
    pub steps: u32,
    pub respawned: bool,
}

impl FrameLatch {
    pub fn sample(&mut self, fresh: TickInput) {
        let held = self.pending;
        self.pending = fresh;
        self.pending.motor.jump |= held.motor.jump;
        self.pending.motor.roll |= held.motor.roll;
        self.pending.motor.fly_toggle |= held.motor.fly_toggle;
        self.pending.attack |= held.attack;
        self.pending.lock_on |= held.lock_on;
        self.pending.dig |= held.dig;
        self.pending.place |= held.place;
        self.pending.respawn |= held.respawn;
        self.pending.look_dx += held.look_dx;
        self.pending.look_dy += held.look_dy;
    }

    pub fn catch_up(&mut self, sim: &mut Greybox, frame_dt: f32) -> CatchUp {
        let outcome = sim.catch_up(frame_dt, self.pending);
        if outcome.steps > 0 {
            self.pending.motor.jump = false;
            self.pending.motor.roll = false;
            self.pending.motor.fly_toggle = false;
            self.pending.attack = false;
            self.pending.lock_on = false;
            self.pending.dig = false;
            self.pending.place = false;
            self.pending.respawn = false;
            self.pending.look_dx = 0.0;
            self.pending.look_dy = 0.0;
        }
        outcome
    }
}

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
    /// Fate-scars. Fixed-point, ticked once per simulation step.
    pub scars: ScarField,
    /// Meshes from the first sync, moved out by the window once a device exists.
    pub initial_uploads: Vec<ChunkUpload>,
    /// Negative skips streaming so a hand-built island is left alone.
    view_radius: i32,
    spawn: Vec3,
    /// Leftover frame time, in seconds. Rendering does not change this except
    /// by asking `catch_up` to spend it.
    accumulator: f64,
    pub tick: u64,
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
            scars: ScarField::new(),
            accumulator: 0.0,
            tick: 0,
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
            scars: ScarField::new(),
            accumulator: 0.0,
            tick: 0,
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
            self.player.facing_yaw = crate::detmath::atan2(wish.z, wish.x);
        }
        self.player.tick(&mut self.physics, wish, FIXED_DT);
        self.camera.pivot = self.player_pos();
    }

    /// Spend wall-clock time as whole 60 Hz ticks. `frame_dt` chooses the
    /// count. It is not the integration delta.
    pub fn catch_up(&mut self, frame_dt: f32, input: TickInput) -> CatchUp {
        let frame_dt = frame_dt.clamp(0.0, MAX_FRAME_DT) as f64;
        self.accumulator += frame_dt;
        let mut steps = 0;
        let mut respawned = false;
        let mut pass_edges = true;
        let step_dt = f64::from(FIXED_DT);
        while self.accumulator >= step_dt && steps < MAX_SUBSTEPS {
            self.accumulator -= step_dt;
            let mut tick = input;
            if !pass_edges {
                tick.motor.jump = false;
                tick.motor.roll = false;
                tick.motor.fly_toggle = false;
                tick.attack = false;
                tick.lock_on = false;
                tick.dig = false;
                tick.place = false;
                tick.respawn = false;
                tick.look_dx = 0.0;
                tick.look_dy = 0.0;
            }
            pass_edges = false;
            if self.step(tick) {
                respawned = true;
            }
            steps += 1;
        }
        if steps == MAX_SUBSTEPS {
            self.accumulator = self.accumulator.min(step_dt);
        }
        CatchUp { steps, respawned }
    }

    /// Advance exactly one fixed tick from a recorded input.
    pub fn step(&mut self, input: TickInput) -> bool {
        self.camera
            .apply_look(input.look_dx, input.look_dy, LOOK_SENSITIVITY);
        if input.attack {
            self.combat.attack.start();
        }
        self.combat.blocking = input.block;
        if input.lock_on {
            self.combat.toggle_lock(&self.physics, &self.player);
        }
        self.tick_controls(input.motor);
        if input.dig {
            let uploads = self.dig_ahead(2.5);
            self.pending_uploads.extend(uploads);
        } else if input.place {
            let material = input.place_material.max(1);
            let uploads = self.place_ahead(1.1, material);
            self.pending_uploads.extend(uploads);
        }
        let respawned = input.respawn && self.combat.player_hp <= 0.0;
        if respawned {
            self.respawn();
        }
        self.tick = self.tick.saturating_add(1);
        respawned
    }

    /// Bit-stable hash of the gameplay state. Iteration order is sorted, and
    /// the hash does not read the clock or the save-file id.
    pub fn state_hash(&self) -> u64 {
        let mut hash = Canon::new();
        hash.u64(self.tick);
        hash.vec3(self.player_pos());
        if let Some(velocity) = self.physics.get_velocity(self.player.body) {
            hash.vec3(velocity);
        }
        if let Some(ctrl) = self.physics.char_map.get(&self.player.body) {
            hash.f32(ctrl.vertical_velocity);
            hash.f32(ctrl.gravity_scale);
        }
        hash.f32(self.player.facing_yaw);
        hash.f32(self.camera.yaw);
        hash.f32(self.camera.pitch);
        hash.f32(self.motor.stamina);
        hash.u8(mode_tag(self.motor.mode));
        hash.u8(self.motor.jumps_used);
        hash.f32(self.motor.roll_time);
        hash.vec3(self.motor.roll_dir);
        hash.bool(self.motor.grounded);
        hash.bool(self.motor.crouching);
        hash.bool(self.motor.sprinting);
        hash.bool(self.motor.invulnerable);
        hash.f32(self.combat.player_hp);
        hash.f32(self.combat.attack.time);
        hash.bool(self.combat.attack.swinging);
        hash.bool(self.combat.attack.hit_landed);
        hash.bool(self.combat.blocking);
        hash.f32(self.combat.hit_react);
        hash.u64(
            self.combat
                .lock
                .map(|index| index as u64)
                .unwrap_or(u64::MAX),
        );
        hash.u64(self.combat.enemies.len() as u64);
        for enemy in &self.combat.enemies {
            hash.u64(enemy.body);
            hash.f32(enemy.health);
            hash.f32(enemy.stagger);
            hash.f32(enemy.windup);
            hash.f32(enemy.attack_cooldown);
            hash.f32(enemy.attack.time);
            hash.bool(enemy.attack.swinging);
            hash.bool(enemy.alive());
            hash.bool(enemy.closing);
            if let Some(pos) = crate::combat::body_pos(&self.physics, enemy.body) {
                hash.vec3(pos);
            }
            if let Some(velocity) = self.physics.get_velocity(enemy.body) {
                hash.vec3(velocity);
            }
        }
        hash.u8(loco_tag(self.graph.state));
        hash.f32(self.graph.time);
        hash.u64(self.ground.edits.len() as u64);
        for edit in &self.ground.edits {
            hash.i32(edit.x);
            hash.i32(edit.y);
            hash.i32(edit.z);
            hash.f32(edit.density);
            hash.u64(u64::from(edit.material));
        }
        let bodies = self.ground.sorted_body_ids();
        hash.u64(bodies.len() as u64);
        for (x, y, z, id) in bodies {
            hash.i32(x);
            hash.i32(y);
            hash.i32(z);
            hash.u64(id);
        }
        hash.finish()
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
        self.combat.tick(
            &mut self.physics,
            &self.player,
            &self.motor,
            FIXED_DT,
            Some(&mut self.scars),
        );
        self.scars.tick();
        if let Some(index) = self.combat.lock {
            if let Some(enemy) = self.combat.enemies.get(index) {
                if let Some(pos) = crate::combat::body_pos(&self.physics, enemy.body) {
                    let to = pos - self.player_pos();
                    self.camera.yaw = crate::detmath::atan2(to.z, to.x);
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
            scars: self.scars.clone(),
        }
    }

    pub fn restore(&mut self, checkpoint: crate::save::Checkpoint) -> Vec<ChunkUpload> {
        self.motor.stamina = checkpoint.stamina.clamp(0.0, self.motor.stamina_max);
        self.combat.player_hp = checkpoint.player_hp.max(0.0);
        self.player.facing_yaw = checkpoint.yaw;
        self.scars = checkpoint.scars;
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

fn mode_tag(mode: crate::motor::MotorMode) -> u8 {
    match mode {
        crate::motor::MotorMode::Locomotion => 0,
        crate::motor::MotorMode::Roll => 1,
        crate::motor::MotorMode::Fly => 2,
    }
}

fn loco_tag(state: crate::locomotion::LocoState) -> u8 {
    use crate::locomotion::LocoState;
    match state {
        LocoState::Idle => 0,
        LocoState::Walk => 1,
        LocoState::Run => 2,
        LocoState::Sprint => 3,
        LocoState::CrouchIdle => 4,
        LocoState::CrouchMove => 5,
        LocoState::Jump => 6,
        LocoState::Roll => 7,
        LocoState::Fly => 8,
        LocoState::Attack => 9,
        LocoState::Block => 10,
        LocoState::Hit => 11,
        LocoState::Death => 12,
    }
}

struct Canon(u64);

impl Canon {
    fn new() -> Self {
        Self(0xcbf29ce484222325)
    }

    fn byte(&mut self, value: u8) {
        self.0 ^= u64::from(value);
        self.0 = self.0.wrapping_mul(0x100000001b3);
    }

    fn bytes(&mut self, data: &[u8]) {
        for value in data {
            self.byte(*value);
        }
    }

    fn f32(&mut self, value: f32) {
        self.bytes(&value.to_bits().to_le_bytes());
    }

    fn u64(&mut self, value: u64) {
        self.bytes(&value.to_le_bytes());
    }

    fn i32(&mut self, value: i32) {
        self.bytes(&value.to_le_bytes());
    }

    fn u8(&mut self, value: u8) {
        self.byte(value);
    }

    fn bool(&mut self, value: bool) {
        self.byte(u8::from(value));
    }

    fn vec3(&mut self, value: Vec3) {
        self.f32(value.x);
        self.f32(value.y);
        self.f32(value.z);
    }

    fn finish(self) -> u64 {
        self.0
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
        assert_eq!(name, "Melee_1H_Attack_Slice_Horizontal");
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
        assert_ne!(clip, "Death_A");
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

    fn play(script: &[TickInput]) -> Greybox {
        let mut sim = Greybox::open(0);
        sim.spawn_enemy(Vec3::new(22.0, 16.0, 16.0));
        sim.spawn_enemy(Vec3::new(12.0, 16.0, 20.0));
        for input in script {
            sim.step(*input);
        }
        sim
    }

    fn recorded_script() -> Vec<TickInput> {
        let mut script = Vec::new();
        for _ in 0..40 {
            script.push(TickInput::default());
        }
        for index in 0..50 {
            script.push(TickInput {
                motor: MotorInput {
                    move_y: 1.0,
                    move_x: if index > 30 { 0.5 } else { 0.0 },
                    sprint: index < 25,
                    jump: index == 8,
                    roll: index == 36,
                    ..MotorInput::default()
                },
                attack: index == 18,
                block: (20..28).contains(&index),
                dig: index == 44,
                lock_on: index == 12,
                look_dx: if index == 4 { 12.0 } else { 0.0 },
                ..TickInput::default()
            });
        }
        script.push(TickInput {
            place: true,
            place_material: 2,
            motor: MotorInput {
                move_y: 1.0,
                ..MotorInput::default()
            },
            ..TickInput::default()
        });
        for _ in 0..30 {
            script.push(TickInput {
                motor: MotorInput {
                    move_x: -1.0,
                    walk: true,
                    ..MotorInput::default()
                },
                ..TickInput::default()
            });
        }
        script.push(TickInput {
            motor: MotorInput {
                fly_toggle: true,
                ..MotorInput::default()
            },
            ..TickInput::default()
        });
        for _ in 0..160 {
            script.push(TickInput {
                motor: MotorInput {
                    move_y: 1.0,
                    ..MotorInput::default()
                },
                ..TickInput::default()
            });
        }
        script
    }

    #[test]
    fn replay_matches_golden_hash() {
        let sim = play(&recorded_script());
        let got = format!("{:016x}", sim.state_hash());
        let golden = include_str!("../golden/replay.hash").trim();
        assert_eq!(
            got, golden,
            "replay state hash {got} does not match the checked-in golden {golden}"
        );
    }

    #[test]
    fn replay_matches_across_independent_runs() {
        let script = recorded_script();
        let first = play(&script);
        let second = play(&script);
        let third = play(&script);
        assert_eq!(first.tick, second.tick);
        assert_eq!(first.state_hash(), second.state_hash());
        assert_eq!(first.state_hash(), third.state_hash());
        assert!(!first.ground.edits.is_empty(), "the script did not dig");

        let dir = tempfile::tempdir().unwrap();
        crate::save::save_checkpoint(dir.path(), &first.checkpoint()).unwrap();
        let loaded = crate::save::load_checkpoint(dir.path()).unwrap();
        assert_eq!(loaded.position.x.to_bits(), first.player_pos().x.to_bits());
        assert_eq!(loaded.position.y.to_bits(), first.player_pos().y.to_bits());
        assert_eq!(loaded.position.z.to_bits(), first.player_pos().z.to_bits());
        assert_eq!(loaded.yaw.to_bits(), first.player.facing_yaw.to_bits());
        assert_eq!(loaded.stamina.to_bits(), first.motor.stamina.to_bits());
        assert_eq!(loaded.player_hp.to_bits(), first.combat.player_hp.to_bits());
        assert_eq!(loaded.edits.len(), first.ground.edits.len());
        for (saved, live) in loaded.edits.iter().zip(first.ground.edits.iter()) {
            assert_eq!(saved.density.to_bits(), live.density.to_bits());
            assert_eq!(saved.material, live.material);
        }

        let mut restored_a = Greybox::open(0);
        let mut restored_b = Greybox::open(0);
        let _ = restored_a.restore(crate::save::load_checkpoint(dir.path()).unwrap());
        let _ = restored_b.restore(crate::save::load_checkpoint(dir.path()).unwrap());
        assert_eq!(restored_a.state_hash(), restored_b.state_hash());
    }

    #[test]
    fn frame_schedule_does_not_change_fixed_ticks() {
        let held = TickInput {
            motor: MotorInput {
                move_y: 1.0,
                sprint: true,
                ..MotorInput::default()
            },
            ..TickInput::default()
        };
        let run = |frames: &[(f32, TickInput)]| {
            let mut sim = Greybox::open(0);
            let mut latch = FrameLatch::default();
            for (dt, input) in frames {
                latch.sample(*input);
                latch.catch_up(&mut sim, *dt);
            }
            sim
        };
        let steady: Vec<(f32, TickInput)> = (0..90).map(|_| (FIXED_DT, held)).collect();
        let doubled: Vec<(f32, TickInput)> = (0..45).map(|_| (FIXED_DT * 2.0, held)).collect();
        let halved: Vec<(f32, TickInput)> = (0..180).map(|_| (FIXED_DT * 0.5, held)).collect();
        let a = run(&steady);
        let b = run(&doubled);
        let c = run(&halved);
        assert_eq!(a.tick, 90);
        assert_eq!(b.tick, a.tick);
        assert_eq!(c.tick, a.tick);
        assert_eq!(a.state_hash(), b.state_hash());
        assert_eq!(a.state_hash(), c.state_hash());
    }

    #[test]
    fn jump_pressed_on_a_short_frame_lands_on_the_next_tick() {
        let mut settled = Greybox::open(0);
        for _ in 0..120 {
            settled.step(TickInput::default());
        }
        assert!(
            settled.player.is_grounded(&settled.physics),
            "need ground before the jump, y={}",
            settled.player_pos().y
        );
        let hash_before = settled.state_hash();

        let mut early = settled;
        let mut late = {
            let mut sim = Greybox::open(0);
            for _ in 0..120 {
                sim.step(TickInput::default());
            }
            sim
        };
        assert_eq!(early.state_hash(), late.state_hash());

        let jump = TickInput {
            motor: MotorInput {
                jump: true,
                ..MotorInput::default()
            },
            ..TickInput::default()
        };
        let mut early_latch = FrameLatch::default();
        early_latch.sample(jump);
        let skipped = early_latch.catch_up(&mut early, FIXED_DT * 0.4);
        assert_eq!(skipped.steps, 0);
        assert_eq!(early.state_hash(), hash_before);
        early_latch.sample(TickInput::default());
        let caught = early_latch.catch_up(&mut early, FIXED_DT * 0.6);
        assert_eq!(caught.steps, 1);

        let mut late_latch = FrameLatch::default();
        late_latch.sample(jump);
        let spent = late_latch.catch_up(&mut late, FIXED_DT);
        assert_eq!(spent.steps, 1);
        assert_eq!(early.tick, late.tick);
        assert_eq!(early.state_hash(), late.state_hash());
        assert_ne!(early.state_hash(), hash_before);
    }

    #[test]
    fn crossing_a_chunk_matches_across_runs() {
        let fly = || {
            let mut sim = Greybox::open(0);
            sim.step(TickInput {
                motor: MotorInput {
                    fly_toggle: true,
                    ..MotorInput::default()
                },
                ..TickInput::default()
            });
            for _ in 0..200 {
                sim.step(TickInput {
                    motor: MotorInput {
                        move_y: 1.0,
                        ..MotorInput::default()
                    },
                    ..TickInput::default()
                });
            }
            sim
        };
        let a = fly();
        let b = fly();
        assert!(a.player_pos().x > 32.0, "did not leave the spawn chunk");
        assert_eq!(a.ground.sorted_body_ids(), b.ground.sorted_body_ids());
        assert_eq!(a.state_hash(), b.state_hash());
    }

    #[test]
    fn an_enemy_death_saves_and_loads_the_same_scar_hash() {
        let mut sim = Greybox::open(0);
        sim.spawn_enemy(Vec3::new(8.0, 6.0, 8.0));
        sim.physics.step();
        sim.combat.enemies[0].health = 0.0;
        sim.tick_controls(MotorInput::default());
        assert!(sim.scars.occupied_count() >= 1);
        assert!(sim.combat.enemies[0].scarred);
        let hash = sim.scars.state_hash();
        let dir = tempfile::tempdir().unwrap();
        crate::save::save_checkpoint(dir.path(), &sim.checkpoint()).unwrap();
        sim.scars = astraweave_veil::ScarField::new();
        let loaded = crate::save::load_checkpoint(dir.path()).unwrap();
        let _ = sim.restore(loaded);
        assert_eq!(sim.scars.state_hash(), hash);
    }
}
