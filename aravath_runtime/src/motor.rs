//! Player motor. Stamina lives here, not on `astraweave-gameplay::Stats`.

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

    /// Advance one fixed step. Returns the horizontal wish passed to the character controller.
    pub fn tick(
        &mut self,
        physics: &mut PhysicsWorld,
        player: &mut Player,
        camera_yaw: f32,
        input: MotorInput,
        dt: f32,
    ) -> Vec3 {
        if input.fly_toggle {
            self.mode = if self.mode == MotorMode::Fly {
                MotorMode::Locomotion
            } else {
                MotorMode::Fly
            };
            self.roll_time = 0.0;
        }

        if let Some(ctrl) = physics.char_map.get_mut(&player.body) {
            ctrl.gravity_scale = if self.mode == MotorMode::Fly {
                0.0
            } else {
                1.0
            };
        }

        let grounded = on_ground(physics, player.body);
        self.grounded = grounded;
        if grounded && self.mode != MotorMode::Roll {
            self.jumps_used = 0;
        }

        if self.mode == MotorMode::Roll {
            self.roll_time += dt;
            self.invulnerable =
                self.roll_time >= ROLL_IFRAME_START && self.roll_time <= ROLL_IFRAME_END;
            let wish = self.roll_dir * ROLL_SPEED;
            if self.roll_time >= ROLL_DURATION {
                self.mode = MotorMode::Locomotion;
                self.invulnerable = false;
                self.roll_time = 0.0;
            }
            player.facing_yaw = crate::detmath::atan2(wish.z, wish.x);
            physics.control_character(player.body, wish, dt, false);
            return wish;
        }

        self.invulnerable = false;
        self.crouching = input.crouch && self.mode != MotorMode::Fly;
        let moving = input.move_x.abs() + input.move_y.abs() > 0.05;
        self.sprinting = input.sprint && moving && self.stamina > 0.0 && !self.crouching;
        self.walking = input.walk && moving && !self.sprinting && !self.crouching;

        if input.roll && self.mode != MotorMode::Fly && self.stamina >= ROLL_COST {
            self.stamina -= ROLL_COST;
            self.mode = MotorMode::Roll;
            self.roll_time = 0.0;
            let (forward, right) = player::flat_basis(camera_yaw);
            let mut dir = forward * input.move_y + right * input.move_x;
            if dir.length_squared() < 1e-4 {
                dir = player::flat_basis(player.facing_yaw).0;
            }
            self.roll_dir = Vec3::new(dir.x, 0.0, dir.z).normalize_or_zero();
            if self.roll_dir.length_squared() < 1e-4 {
                self.roll_dir = Vec3::X;
            }
        }

        if input.jump && self.mode != MotorMode::Roll {
            if self.mode == MotorMode::Fly {
                if let Some(ctrl) = physics.char_map.get_mut(&player.body) {
                    ctrl.vertical_velocity = FLY_SPEED;
                }
            } else if grounded && self.jumps_used == 0 {
                physics.jump(player.body, JUMP_HEIGHT);
                self.jumps_used = 1;
            } else if self.jumps_used == 1 {
                if let Some(ctrl) = physics.char_map.get_mut(&player.body) {
                    ctrl.vertical_velocity = crate::detmath::sqrt(2.0 * 9.81 * JUMP_HEIGHT);
                }
                self.jumps_used = 2;
            }
        } else if self.mode == MotorMode::Fly && input.crouch {
            if let Some(ctrl) = physics.char_map.get_mut(&player.body) {
                ctrl.vertical_velocity = -FLY_SPEED;
            }
        } else if self.mode == MotorMode::Fly {
            if let Some(ctrl) = physics.char_map.get_mut(&player.body) {
                ctrl.vertical_velocity = 0.0;
            }
        }

        let speed = if self.mode == MotorMode::Fly {
            FLY_SPEED
        } else if self.crouching {
            CROUCH_SPEED
        } else if self.sprinting {
            SPRINT_SPEED
        } else if self.walking {
            WALK_SPEED
        } else if moving {
            RUN_SPEED
        } else {
            0.0
        };
        let (forward, right) = player::flat_basis(camera_yaw);
        let mut wish = player::wish_from_axes(forward, right, input.move_x, input.move_y, speed);
        if !moving {
            wish = Vec3::ZERO;
        }
        if input.feet_locked && self.mode != MotorMode::Fly {
            wish = Vec3::ZERO;
        }
        if !grounded && self.mode == MotorMode::Locomotion {
            wish *= 0.4;
        }
        if wish.length_squared() > 1e-6 {
            player.facing_yaw = crate::detmath::atan2(wish.z, wish.x);
        }

        if self.sprinting {
            self.stamina = (self.stamina - SPRINT_DRAIN * dt).max(0.0);
        } else if self.mode != MotorMode::Roll && grounded {
            self.stamina = (self.stamina + STAMINA_REGEN * dt).min(self.stamina_max);
        }

        physics.control_character(player.body, wish, dt, false);
        wish
    }
}

fn on_ground(physics: &PhysicsWorld, body: u64) -> bool {
    physics
        .char_map
        .get(&body)
        .is_some_and(|c| c.time_since_grounded <= 0.02)
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
        let player = Player::spawn(&mut physics, Vec3::new(0.0, 2.0, 0.0));
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
        motor.tick(&mut physics, &mut player, 0.0, MotorInput::default(), 0.1);
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
        assert!((dry.stamina - 10.0).abs() < 0.01);
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
}
