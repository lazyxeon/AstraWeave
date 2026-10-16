//! Player motor. Stamina lives here, not on `astraweave-gameplay::Stats`.

use astraweave_physics::traversal::{
    self, ProbeFrame, TraversalBody, TraversalInput, TraversalMode, TICK_DT,
};
use astraweave_physics::PhysicsWorld;
use glam::Vec3;

use crate::player::{self, Player};

pub const WALK_SPEED: f32 = 3.0;
pub const RUN_SPEED: f32 = 5.0;
pub const SPRINT_SPEED: f32 = 8.0;
pub const CROUCH_SPEED: f32 = 1.8;
pub const FLY_SPEED: f32 = 8.0;
pub const ROLL_SPEED: f32 = 9.0;
pub const ROLL_DURATION: f32 = 0.5;
pub const ROLL_IFRAME_START: f32 = 0.08;
pub const ROLL_IFRAME_END: f32 = 0.32;
pub const ROLL_COST: f32 = 25.0;
pub const SPRINT_DRAIN: f32 = 18.0;
pub const STAMINA_REGEN: f32 = 22.0;
pub const JUMP_HEIGHT: f32 = 1.15;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MotorMode {
    Locomotion,
    Roll,
    Fly,
}

#[derive(Clone, Debug)]
pub struct Motor {
    pub stamina: f32,
    pub stamina_max: f32,
    pub mode: MotorMode,
    pub roll_time: f32,
    pub roll_dir: Vec3,
    pub jumps_used: u8,
    pub crouching: bool,
    pub sprinting: bool,
    pub walking: bool,
    pub grounded: bool,
    pub invulnerable: bool,
    body: TraversalBody,
    synced: bool,
}

impl Default for Motor {
    fn default() -> Self {
        Self {
            stamina: 100.0,
            stamina_max: 100.0,
            mode: MotorMode::Locomotion,
            roll_time: 0.0,
            roll_dir: Vec3::X,
            jumps_used: 0,
            crouching: false,
            sprinting: false,
            walking: false,
            grounded: true,
            invulnerable: false,
            body: TraversalBody::default(),
            synced: false,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct MotorInput {
    pub move_x: f32,
    pub move_y: f32,
    pub jump: bool,
    pub sprint: bool,
    pub crouch: bool,
    pub roll: bool,
    pub fly_toggle: bool,
    pub walk: bool,
    pub feet_locked: bool,
    pub climb: bool,
    pub glide: bool,
    pub dive: bool,
    pub prone: bool,
}

impl Motor {
    pub fn apply_damage_multiplier(&self, blocking: bool) -> f32 {
        if self.invulnerable {
            0.0
        } else if blocking {
            0.2
        } else {
            1.0
        }
    }

    /// Advance one fixed 60 Hz tick. `dt` is ignored; callers pass [`TICK_DT`].
    pub fn tick(
        &mut self,
        physics: &mut PhysicsWorld,
        player: &mut Player,
        camera_yaw: f32,
        input: MotorInput,
        dt: f32,
    ) -> Vec3 {
        let _ = dt;
        if input.fly_toggle {
            self.mode = if self.mode == MotorMode::Fly {
                MotorMode::Locomotion
            } else {
                MotorMode::Fly
            };
            self.roll_time = 0.0;
            self.synced = false;
        }

        if self.mode == MotorMode::Fly {
            return self.tick_fly(physics, player, camera_yaw, input);
        }

        self.sync_feet(physics, player);
        let probe = ground_probe(physics, player.body);
        let (move_x, move_y) = if input.feet_locked {
            (0.0, 0.0)
        } else {
            (input.move_x, input.move_y)
        };
        traversal::step(
            &mut self.body,
            TraversalInput {
                move_x,
                move_y,
                yaw: camera_yaw,
                jump: input.jump,
                roll: input.roll,
                sprint: input.sprint,
                crouch: input.crouch,
                walk: input.walk,
                climb: input.climb,
                glide: input.glide,
                dive: input.dive,
                prone: input.prone,
            },
            &probe,
            0,
        );
        self.publish(physics, player);
        Vec3::new(self.body.vel.x, 0.0, self.body.vel.z)
    }

    /// One tick on deformable terrain. Notices and climb flags come from the
    /// destruction world. The capsule is placed from the resulting feet.
    pub fn tick_deformed(
        &mut self,
        physics: &mut PhysicsWorld,
        player: &mut Player,
        camera_yaw: f32,
        input: MotorInput,
        stamp: &astraweave_physics::deformed::ColumnStamp,
        notices: &[astraweave_physics::deformed::GeometryNotice],
        anchor: &mut [i32; 3],
        attached: &mut bool,
        climb_flags: astraweave_physics::deformed::FractureFlags,
    ) -> Vec3 {
        self.sync_feet(physics, player);
        let (move_x, move_y) = if input.feet_locked {
            (0.0, 0.0)
        } else {
            (input.move_x, input.move_y)
        };
        astraweave_physics::deformed::deformed_step(
            &mut self.body,
            TraversalInput {
                move_x,
                move_y,
                yaw: camera_yaw,
                jump: input.jump,
                roll: input.roll,
                sprint: input.sprint,
                crouch: input.crouch,
                walk: input.walk,
                climb: input.climb,
                glide: input.glide,
                dive: input.dive,
                prone: input.prone,
            },
            stamp,
            notices,
            anchor,
            attached,
            climb_flags,
            0,
        );
        self.publish(physics, player);
        Vec3::new(self.body.vel.x, 0.0, self.body.vel.z)
    }

    fn tick_fly(
        &mut self,
        physics: &mut PhysicsWorld,
        player: &mut Player,
        camera_yaw: f32,
        input: MotorInput,
    ) -> Vec3 {
        if let Some(ctrl) = physics.char_map.get_mut(&player.body) {
            ctrl.gravity_scale = 0.0;
            ctrl.vertical_velocity = if input.jump {
                FLY_SPEED
            } else if input.crouch {
                -FLY_SPEED
            } else {
                0.0
            };
        }
        let moving = input.move_x.abs() + input.move_y.abs() > 0.05;
        let (forward, right) = player::flat_basis(camera_yaw);
        let mut wish = if moving {
            player::wish_from_axes(forward, right, input.move_x, input.move_y, FLY_SPEED)
        } else {
            Vec3::ZERO
        };
        if input.feet_locked {
            wish = Vec3::ZERO;
        }
        if wish.length_squared() > 1e-6 {
            player.facing_yaw = crate::detmath::atan2(wish.z, wish.x);
        }
        self.grounded = false;
        self.crouching = false;
        self.sprinting = false;
        self.invulnerable = false;
        physics.control_character(player.body, wish, TICK_DT, false);
        wish
    }

    fn sync_feet(&mut self, physics: &PhysicsWorld, player: &Player) {
        if self.synced {
            return;
        }
        let center = player.translation(physics).unwrap_or(Vec3::ZERO);
        let half = capsule_half(physics, player.body);
        self.body.pos = Vec3::new(center.x, center.y - half, center.z);
        self.body.stamina = self.stamina;
        self.body.stamina_max = self.stamina_max;
        self.body.facing_yaw = player.facing_yaw;
        self.synced = true;
    }

    fn publish(&mut self, physics: &mut PhysicsWorld, player: &mut Player) {
        self.stamina = self.body.stamina;
        self.jumps_used = self.body.jumps_used;
        self.invulnerable = self.body.invulnerable;
        self.crouching = self.body.crouching;
        self.sprinting = self.body.sprinting;
        self.walking = self.body.walking;
        self.grounded = self.body.mode != TraversalMode::Air;
        self.roll_dir = self.body.roll_dir;
        self.roll_time = f32::from(self.body.roll_tick);
        self.mode = if self.body.mode == TraversalMode::Roll {
            MotorMode::Roll
        } else {
            MotorMode::Locomotion
        };
        player.facing_yaw = self.body.facing_yaw;
        let half = capsule_half(physics, player.body);
        let center = self.body.pos + Vec3::Y * half;
        physics.place_character(player.body, center);
        if let Some(ctrl) = physics.char_map.get_mut(&player.body) {
            ctrl.vertical_velocity = self.body.vel.y;
            ctrl.gravity_scale = 1.0;
            ctrl.time_since_grounded = if self.grounded { 0.0 } else { TICK_DT };
        }
    }
}

fn capsule_half(physics: &PhysicsWorld, body: u64) -> f32 {
    physics
        .char_map
        .get(&body)
        .map(|ctrl| ctrl.height * 0.5 + ctrl.radius)
        .unwrap_or(1.0)
}

fn ground_probe(physics: &PhysicsWorld, body: u64) -> ProbeFrame {
    let mut probe = match physics.character_ground(body) {
        Some((ground_y, normal_y)) => {
            let mut probe = ProbeFrame::plane(ground_y);
            probe.support = normal_y > 0.35;
            probe.normal_y = normal_y;
            probe
        }
        None => {
            let mut probe = ProbeFrame::plane(0.0);
            probe.support = false;
            probe
        }
    };
    if let Some(center) = physics.body_transform(body).map(|m| m.w_axis.truncate()) {
        probe.water_surface = physics.water_surface_at(center);
        probe.climbability = physics.climbability_at(body, center);
    }
    probe
}

#[cfg(test)]
mod tests {
    use super::*;
    use astraweave_physics::PhysicsWorld;
    use glam::Vec3;

    fn grounded_player() -> (PhysicsWorld, Player) {
        let mut physics = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));
        let _ = physics.create_ground_plane(Vec3::new(20.0, 0.5, 20.0), 1.0);
        physics.step();
        let player = Player::spawn(&mut physics, Vec3::new(0.0, 1.25, 0.0));
        physics.place_character(player.body, Vec3::new(0.0, 1.25, 0.0));
        physics.step();
        (physics, player)
    }

    #[test]
    fn roll_iframes_block_damage_and_empty_stamina_refuses_roll() {
        let (mut physics, mut player) = grounded_player();
        let mut motor = Motor::default();
        motor.tick(
            &mut physics,
            &mut player,
            0.0,
            MotorInput {
                roll: true,
                move_y: 1.0,
                ..MotorInput::default()
            },
            0.016,
        );
        assert_eq!(motor.mode, MotorMode::Roll);
        assert!(!motor.invulnerable);
        for _ in 0..5 {
            motor.tick(
                &mut physics,
                &mut player,
                0.0,
                MotorInput::default(),
                TICK_DT,
            );
        }
        assert!(motor.invulnerable);
        assert_eq!(motor.apply_damage_multiplier(false), 0.0);

        let mut dry = Motor {
            stamina: 10.0,
            ..Motor::default()
        };
        dry.tick(
            &mut physics,
            &mut player,
            0.0,
            MotorInput {
                roll: true,
                ..MotorInput::default()
            },
            0.016,
        );
        assert_ne!(dry.mode, MotorMode::Roll);
        assert!(
            dry.stamina + 1e-3 >= 10.0,
            "a refused roll must not spend stamina, got {}",
            dry.stamina
        );
    }

    #[test]
    fn air_control_is_only_a_fraction_of_the_run() {
        let (mut physics, mut player) = grounded_player();
        let mut motor = Motor::default();
        motor.tick(
            &mut physics,
            &mut player,
            0.0,
            MotorInput {
                jump: true,
                move_y: 1.0,
                ..MotorInput::default()
            },
            1.0 / 60.0,
        );
        physics.step();
        for _ in 0..10 {
            motor.tick(
                &mut physics,
                &mut player,
                0.0,
                MotorInput {
                    move_y: 1.0,
                    ..MotorInput::default()
                },
                1.0 / 60.0,
            );
            physics.step();
        }
        assert!(!motor.grounded);
        let air = motor.tick(
            &mut physics,
            &mut player,
            0.0,
            MotorInput {
                move_y: 1.0,
                ..MotorInput::default()
            },
            1.0 / 60.0,
        );
        assert!(
            air.length() < RUN_SPEED * 0.5,
            "air wish {air:?} should be well under run speed"
        );
    }

    #[test]
    fn walk_is_slower_than_run() {
        let (mut physics, mut player) = grounded_player();
        let mut walking = Motor::default();
        let walk = walking.tick(
            &mut physics,
            &mut player,
            0.0,
            MotorInput {
                move_y: 1.0,
                walk: true,
                ..MotorInput::default()
            },
            0.016,
        );
        let mut running = Motor::default();
        let run = running.tick(
            &mut physics,
            &mut player,
            0.0,
            MotorInput {
                move_y: 1.0,
                ..MotorInput::default()
            },
            0.016,
        );
        assert!((walk.length() - WALK_SPEED).abs() < 0.05, "{walk:?}");
        assert!((run.length() - RUN_SPEED).abs() < 0.05, "{run:?}");
    }

    #[test]
    fn stamina_does_not_regen_during_roll() {
        let (mut physics, mut player) = grounded_player();
        let mut motor = Motor {
            stamina: 40.0,
            ..Motor::default()
        };
        motor.tick(
            &mut physics,
            &mut player,
            0.0,
            MotorInput {
                roll: true,
                move_y: 1.0,
                ..MotorInput::default()
            },
            0.016,
        );
        let after_cost = motor.stamina;
        motor.tick(&mut physics, &mut player, 0.0, MotorInput::default(), 0.2);
        assert!(
            (motor.stamina - after_cost).abs() < 0.01,
            "regen during roll: {} -> {}",
            after_cost,
            motor.stamina
        );
    }

    #[test]
    fn double_jump_adds_a_second_impulse() {
        let (mut physics, mut player) = grounded_player();
        let mut motor = Motor::default();
        for _ in 0..40 {
            motor.tick(
                &mut physics,
                &mut player,
                0.0,
                MotorInput::default(),
                1.0 / 60.0,
            );
            physics.step();
        }
        let y0 = player.translation(&physics).unwrap().y;
        motor.tick(
            &mut physics,
            &mut player,
            0.0,
            MotorInput {
                jump: true,
                ..MotorInput::default()
            },
            1.0 / 60.0,
        );
        physics.step();
        assert_eq!(motor.jumps_used, 1);
        for _ in 0..12 {
            motor.tick(
                &mut physics,
                &mut player,
                0.0,
                MotorInput::default(),
                1.0 / 60.0,
            );
            physics.step();
        }
        let y_air = player.translation(&physics).unwrap().y;
        motor.tick(
            &mut physics,
            &mut player,
            0.0,
            MotorInput {
                jump: true,
                ..MotorInput::default()
            },
            1.0 / 60.0,
        );
        physics.step();
        assert_eq!(motor.jumps_used, 2);
        let y1 = player.translation(&physics).unwrap().y;
        assert!(
            y1 >= y_air,
            "double jump did not rise: {y_air} -> {y1} start {y0}"
        );
    }

    #[test]
    fn deformed_notice_detaches_a_climb_on_the_capsule() {
        use astraweave_physics::deformed::{ColumnStamp, FractureFlags, GeometryNotice};
        let (mut physics, mut player) = grounded_player();
        let mut motor = Motor::default();
        motor.body.mode = astraweave_physics::traversal::TraversalMode::Climb;
        motor.body.vel = Vec3::new(1.25, 0.2, 0.0);
        let stamp = ColumnStamp {
            x: 0,
            z: 0,
            heights: [[4; 3]; 3],
        };
        let notice = GeometryNotice {
            tick: 1,
            x0: 0,
            y0: 0,
            z0: 0,
            x1: 2,
            y1: 8,
            z1: 2,
            reason: 2,
        };
        let mut anchor = [0, 4, 0];
        let mut attached = true;
        let wish = motor.tick_deformed(
            &mut physics,
            &mut player,
            0.0,
            MotorInput {
                climb: true,
                ..MotorInput::default()
            },
            &stamp,
            &[notice],
            &mut anchor,
            &mut attached,
            FractureFlags::CLIMBABLE,
        );
        assert!(!attached);
        assert!((wish.x - 1.25).abs() < 1e-4, "wish {wish:?}");
    }
}
