//! Sword, shield, lock-on, and a handful of simple enemies.
//!
//! A hit is a physics ray during the active window. This crate does not call
//! the gameplay sweep or the Manhattan combo ticker.

use astraweave_physics::PhysicsWorld;
use glam::Vec3;

use crate::motor::Motor;
use crate::player::{self, Player};

pub const STARTUP: f32 = 0.10;
pub const ACTIVE: f32 = 0.14;
pub const RECOVERY: f32 = 0.22;
pub const SWORD_REACH: f32 = 1.8;
pub const SWORD_DAMAGE: f32 = 25.0;
pub const ENEMY_DAMAGE: f32 = 12.0;
pub const HIT_REACT: f32 = 0.4;
pub const ATTACK_WINDOW: f32 = STARTUP + ACTIVE + RECOVERY;
/// Past this distance an enemy loses interest and holds still.
pub const AGGRO_RANGE: f32 = 18.0;
pub const MELEE_RANGE: f32 = 2.05;
pub const ATTACK_WINDUP: f32 = 0.32;
pub const ATTACK_COOLDOWN: f32 = 0.75;

#[derive(Clone, Debug)]
pub struct Attack {
    pub time: f32,
    pub swinging: bool,
    pub hit_landed: bool,
}

impl Default for Attack {
    fn default() -> Self {
        Self {
            time: 0.0,
            swinging: false,
            hit_landed: false,
        }
    }
}

impl Attack {
    pub fn start(&mut self) {
        if !self.swinging {
            *self = Self {
                time: 0.0,
                swinging: true,
                hit_landed: false,
            };
        }
    }

    pub fn active(&self) -> bool {
        self.swinging && self.time >= STARTUP && self.time < STARTUP + ACTIVE
    }

    pub fn locks_feet(&self) -> bool {
        self.swinging && self.time < STARTUP + ACTIVE + RECOVERY
    }

    pub fn advance(&mut self, dt: f32) {
        if !self.swinging {
            return;
        }
        self.time += dt;
        if self.time >= STARTUP + ACTIVE + RECOVERY {
            self.swinging = false;
            self.time = 0.0;
        }
    }
}

#[derive(Clone, Debug)]
pub struct Enemy {
    pub body: u64,
    pub health: f32,
    pub attack: Attack,
    pub stagger: f32,
    /// True while this enemy is stepping toward the player.
    pub closing: bool,
    pub anim_time: f32,
    pub down_time: f32,
    /// Time spent in melee range before the next swing.
    pub windup: f32,
    pub attack_cooldown: f32,
}

/// Which figurine clip to bake, and how to read the clock.
pub enum EnemyPose {
    Loop {
        clip: &'static str,
        time: f32,
    },
    /// `phase` is 0..1 across the clip.
    Fraction {
        clip: &'static str,
        phase: f32,
    },
    /// Seconds from the start, held at the last frame.
    Hold {
        clip: &'static str,
        time: f32,
    },
}

impl Enemy {
    pub fn new(body: u64) -> Self {
        Self {
            body,
            health: 40.0,
            attack: Attack::default(),
            stagger: 0.0,
            closing: false,
            anim_time: 0.0,
            down_time: 0.0,
            windup: 0.0,
            attack_cooldown: 0.0,
        }
    }

    pub fn alive(&self) -> bool {
        self.health > 0.0
    }

    pub fn pose(&self) -> EnemyPose {
        if !self.alive() {
            EnemyPose::Hold {
                clip: "die",
                time: self.down_time,
            }
        } else if self.attack.swinging {
            EnemyPose::Fraction {
                clip: "attack-melee-right",
                phase: self.attack.time / ATTACK_WINDOW,
            }
        } else if self.closing {
            EnemyPose::Loop {
                clip: "walk",
                time: self.anim_time,
            }
        } else {
            EnemyPose::Loop {
                clip: "idle",
                time: self.anim_time,
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct Combat {
    pub player_hp: f32,
    pub attack: Attack,
    pub blocking: bool,
    pub enemies: Vec<Enemy>,
    pub lock: Option<usize>,
    pub hit_react: f32,
}

impl Default for Combat {
    fn default() -> Self {
        Self {
            player_hp: 100.0,
            attack: Attack::default(),
            blocking: false,
            enemies: Vec::new(),
            lock: None,
            hit_react: 0.0,
        }
    }
}

impl Combat {
    pub fn toggle_lock(&mut self, physics: &PhysicsWorld, player: &Player) {
        if self.lock.is_some() {
            self.lock = None;
            return;
        }
        let origin = body_pos(physics, player.body).unwrap_or(Vec3::ZERO);
        self.lock = self
            .enemies
            .iter()
            .enumerate()
            .filter(|(_, enemy)| enemy.alive())
            .filter_map(|(index, enemy)| {
                body_pos(physics, enemy.body).map(|pos| (index, (pos - origin).length_squared()))
            })
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(index, _)| index);
    }

    pub fn tick(&mut self, physics: &mut PhysicsWorld, player: &Player, motor: &Motor, dt: f32) {
        let was_active = self.attack.active();
        if was_active && !self.attack.hit_landed {
            if let Some(index) = sword_target(self, physics, player) {
                self.attack.hit_landed = true;
                let enemy = &mut self.enemies[index];
                enemy.health -= SWORD_DAMAGE;
                enemy.stagger = 0.45;
                enemy.attack.swinging = false;
            }
        }
        self.attack.advance(dt);

        let player_pos = body_pos(physics, player.body).unwrap_or(Vec3::ZERO);
        let blocking = self.blocking;
        let player_body = player.body;
        let hp_before = self.player_hp;
        let spots: Vec<(u64, Vec3)> = self
            .enemies
            .iter()
            .filter(|enemy| enemy.alive())
            .filter_map(|enemy| body_pos(physics, enemy.body).map(|pos| (enemy.body, pos)))
            .collect();
        for enemy in &mut self.enemies {
            enemy.anim_time += dt;
            if !enemy.alive() {
                enemy.closing = false;
                enemy.down_time += dt;
                continue;
            }
            let Some(pos) = body_pos(physics, enemy.body) else {
                continue;
            };
            let to_player = player_pos - pos;
            let flat = Vec3::new(to_player.x, 0.0, to_player.z);
            let dist = flat.length();
            if enemy.stagger > 0.0 {
                enemy.stagger = (enemy.stagger - dt).max(0.0);
                enemy.closing = false;
                enemy.windup = 0.0;
                if dist > 0.01 {
                    physics.control_character(enemy.body, -flat.normalize() * 2.4, dt, false);
                }
                enemy.attack.advance(dt);
                continue;
            }
            if dist > AGGRO_RANGE {
                enemy.closing = false;
                enemy.windup = 0.0;
                enemy.attack.advance(dt);
                continue;
            }
            if dist > MELEE_RANGE {
                enemy.closing = true;
                enemy.windup = 0.0;
                let speed = if dist < 7.0 { 4.6 } else { 3.2 };
                let wish = steer(flat, pos, &spots, enemy.body) * speed;
                physics.control_character(enemy.body, wish, dt, false);
            } else {
                enemy.closing = false;
                if !enemy.attack.swinging {
                    if enemy.attack_cooldown > 0.0 {
                        enemy.attack_cooldown = (enemy.attack_cooldown - dt).max(0.0);
                    } else {
                        enemy.windup += dt;
                        if enemy.windup >= ATTACK_WINDUP {
                            enemy.attack.start();
                            enemy.windup = 0.0;
                            enemy.attack_cooldown = ATTACK_COOLDOWN;
                        }
                    }
                }
                if enemy.attack.active() && !enemy.attack.hit_landed && dist > 0.01 {
                    let origin = pos + Vec3::Y * 0.6 + flat.normalize() * 0.7;
                    if let Some((_, _, Some(id), _)) =
                        physics.raycast(origin, flat.normalize(), 1.6)
                    {
                        if id == player_body {
                            let scale = motor.apply_damage_multiplier(blocking);
                            self.player_hp -= ENEMY_DAMAGE * scale;
                            enemy.attack.hit_landed = true;
                        }
                    }
                }
            }
            enemy.attack.advance(dt);
        }
        if self.player_hp < hp_before {
            self.hit_react = HIT_REACT;
        } else {
            self.hit_react = (self.hit_react - dt).max(0.0);
        }
        if self
            .lock
            .is_some_and(|index| self.enemies.get(index).is_none_or(|enemy| !enemy.alive()))
        {
            self.lock = None;
        }
    }
}

pub fn sword_target(combat: &Combat, physics: &PhysicsWorld, player: &Player) -> Option<usize> {
    let (forward, _) = player::flat_basis(player.facing_yaw);
    let origin = body_pos(physics, player.body)? + Vec3::Y * 0.6 + forward * 0.7;
    let (_, _, id, _) = physics.raycast(origin, forward, SWORD_REACH)?;
    let id = id?;
    combat
        .enemies
        .iter()
        .position(|enemy| enemy.alive() && enemy.body == id)
}

/// Approach direction with a push away from nearby allies.
pub fn steer(to_target: Vec3, self_pos: Vec3, others: &[(u64, Vec3)], self_body: u64) -> Vec3 {
    let flat = Vec3::new(to_target.x, 0.0, to_target.z);
    if flat.length_squared() < 1e-6 {
        return Vec3::ZERO;
    }
    let mut wish = flat.normalize();
    let mut push = Vec3::ZERO;
    for (id, pos) in others {
        if *id == self_body {
            continue;
        }
        let away = Vec3::new(self_pos.x - pos.x, 0.0, self_pos.z - pos.z);
        let distance = away.length();
        if distance < 1.6 && distance > 1e-3 {
            push += away.normalize() * (1.6 - distance);
        }
    }
    let mixed = wish + push;
    if mixed.length_squared() > 1e-6 {
        wish = mixed.normalize();
    }
    wish
}

pub fn body_pos(physics: &PhysicsWorld, id: u64) -> Option<Vec3> {
    physics.body_transform(id).map(|m| m.w_axis.truncate())
}

#[cfg(test)]
mod tests {
    use super::*;
    use astraweave_physics::PhysicsWorld;

    fn arena() -> (PhysicsWorld, Player, Combat) {
        let mut physics = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));
        let _ = physics.create_ground_plane(Vec3::new(30.0, 0.5, 30.0), 1.0);
        physics.step();
        let mut player = Player::spawn(&mut physics, Vec3::new(0.0, 1.5, 0.0));
        player.facing_yaw = 0.0;
        let enemy_body =
            physics.add_character(Vec3::new(1.3, 1.5, 0.0), Vec3::new(0.35, 0.8, 0.35));
        physics.step();
        let combat = Combat {
            enemies: vec![Enemy::new(enemy_body)],
            ..Combat::default()
        };
        (physics, player, combat)
    }

    #[test]
    fn swing_outside_the_active_window_misses() {
        let (mut physics, player, mut combat) = arena();
        combat.attack.swinging = true;
        combat.attack.time = 0.02;
        let before = combat.enemies[0].health;
        combat.tick(&mut physics, &player, &Motor::default(), 0.0);
        assert!((combat.enemies[0].health - before).abs() < 0.01);
    }

    #[test]
    fn active_swing_damages_once_and_staggers() {
        let (mut physics, player, mut combat) = arena();
        combat.attack.swinging = true;
        combat.attack.time = STARTUP;
        combat.tick(&mut physics, &player, &Motor::default(), 0.0);
        assert!(combat.attack.hit_landed);
        assert!((combat.enemies[0].health - (40.0 - SWORD_DAMAGE)).abs() < 0.01);
        assert!(combat.enemies[0].stagger > 0.0);
        let health = combat.enemies[0].health;
        combat.attack.time = STARTUP;
        combat.tick(&mut physics, &player, &Motor::default(), 0.0);
        assert!((combat.enemies[0].health - health).abs() < 0.01);
    }

    #[test]
    fn stagger_interrupts_approach() {
        let (mut physics, player, mut combat) = arena();
        combat.enemies[0].stagger = 0.3;
        let before = body_pos(&physics, combat.enemies[0].body).unwrap();
        combat.tick(&mut physics, &player, &Motor::default(), 0.05);
        physics.step();
        let after = body_pos(&physics, combat.enemies[0].body).unwrap();
        assert!(
            after.x > before.x + 0.04,
            "stagger should step away from the player, {before:?} -> {after:?}"
        );
    }

    #[test]
    fn a_distant_enemy_holds_and_a_near_one_waits_before_swinging() {
        let (mut physics, player, mut combat) = arena();
        physics.set_body_position(combat.enemies[0].body, Vec3::new(22.0, 1.5, 0.0));
        physics.step();
        let before = body_pos(&physics, combat.enemies[0].body).unwrap();
        assert!(before.x > AGGRO_RANGE, "teleport did not stick {before:?}");
        combat.tick(&mut physics, &player, &Motor::default(), 0.1);
        physics.step();
        let after = body_pos(&physics, combat.enemies[0].body).unwrap();
        assert!(
            (after - before).length() < 0.2,
            "leash failed {before:?} -> {after:?}"
        );

        let (mut physics, player, mut combat) = arena();
        combat.tick(&mut physics, &player, &Motor::default(), 0.05);
        assert!(!combat.enemies[0].attack.swinging);
        for _ in 0..8 {
            combat.tick(&mut physics, &player, &Motor::default(), 0.05);
        }
        assert!(combat.enemies[0].attack.swinging);
    }

    #[test]
    fn steer_slides_off_a_neighbor() {
        let wish = steer(
            Vec3::X * 10.0,
            Vec3::ZERO,
            &[(1, Vec3::ZERO), (2, Vec3::new(0.0, 0.0, 1.0))],
            1,
        );
        assert!(wish.x > 0.5, "{wish:?}");
        assert!(wish.z < -0.2, "{wish:?}");
    }

    #[test]
    fn enemy_pose_follows_closing_attack_and_death() {
        let mut enemy = Enemy::new(1);
        assert!(matches!(enemy.pose(), EnemyPose::Loop { clip: "idle", .. }));
        enemy.closing = true;
        assert!(matches!(enemy.pose(), EnemyPose::Loop { clip: "walk", .. }));
        enemy.attack.swinging = true;
        assert!(matches!(
            enemy.pose(),
            EnemyPose::Fraction {
                clip: "attack-melee-right",
                ..
            }
        ));
        enemy.health = 0.0;
        assert!(matches!(enemy.pose(), EnemyPose::Hold { clip: "die", .. }));
    }

    #[test]
    fn a_landed_enemy_hit_starts_hit_react() {
        let (mut physics, player, mut combat) = arena();
        combat.enemies[0].attack.swinging = true;
        combat.enemies[0].attack.time = STARTUP;
        let before = combat.player_hp;
        combat.tick(&mut physics, &player, &Motor::default(), 0.0);
        assert!(combat.player_hp < before, "ray missed the player");
        assert!((combat.hit_react - HIT_REACT).abs() < 0.01);
    }

    #[test]
    fn lock_on_clears_when_the_target_dies() {
        let (mut physics, player, mut combat) = arena();
        combat.toggle_lock(&physics, &player);
        assert!(combat.lock.is_some());
        combat.enemies[0].health = 0.0;
        combat.tick(&mut physics, &player, &Motor::default(), 0.016);
        assert!(combat.lock.is_none());
    }
}
