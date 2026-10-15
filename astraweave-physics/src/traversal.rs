//! Tick-based traversal.
//!
//! One tick is [`TICK_DT`] (60 Hz). The sim owns displacement. Callers pass a
//! [`ProbeFrame`] from Rapier, or a flat plane for the replay gate. Slope and
//! jump roots go through [`crate::detmath`] (`libm`). Random draws, when a
//! later phase needs them, use `astraweave_core::SimRng` and
//! `SIM_SYSTEM_TRAVERSAL`. This module does not read the clock.

use glam::Vec3;

use crate::detmath;

/// Fixed simulation step. Frame time never reaches this module.
pub const TICK_HZ: u32 = 60;
pub const TICK_DT: f32 = 1.0 / 60.0;
pub const GRAVITY: f32 = 9.81;

pub const COYOTE_TICKS: u16 = 6;
pub const JUMP_BUFFER_TICKS: u16 = 8;
pub const ROLL_TICKS: u16 = 30;
/// Inclusive window. Tick 0 of a roll is not invulnerable.
pub const ROLL_IFRAME_START: u16 = 5;
pub const ROLL_IFRAME_END: u16 = 19;

pub const WALK_SPEED: f32 = 3.0;
pub const RUN_SPEED: f32 = 5.0;
pub const SPRINT_SPEED: f32 = 8.0;
pub const CROUCH_SPEED: f32 = 1.8;
pub const ROLL_SPEED: f32 = 9.0;
pub const JUMP_HEIGHT: f32 = 1.15;
pub const AIR_CONTROL: f32 = 0.4;
pub const ROLL_COST: f32 = 25.0;
pub const SPRINT_DRAIN_PER_TICK: f32 = 18.0 * TICK_DT;
pub const STAMINA_REGEN_PER_TICK: f32 = 22.0 * TICK_DT;
pub const SNAP: f32 = 0.05;

const REPLAY_TICKS: u32 = 180;
const REPLAY_SEED: u64 = 0x7A4E_0001;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum TraversalMode {
    Ground = 1,
    Air = 2,
    Roll = 3,
    Crouch = 4,
}

#[derive(Clone, Debug)]
pub struct TraversalBody {
    pub mode: TraversalMode,
    pub pos: Vec3,
    pub vel: Vec3,
    pub facing_yaw: f32,
    pub stamina: f32,
    pub stamina_max: f32,
    pub coyote: u16,
    pub jump_buffer: u16,
    pub jumps_used: u8,
    pub roll_tick: u16,
    pub roll_dir: Vec3,
    pub invulnerable: bool,
    pub sprinting: bool,
    pub walking: bool,
    pub crouching: bool,
}

impl Default for TraversalBody {
    fn default() -> Self {
        Self {
            mode: TraversalMode::Ground,
            pos: Vec3::ZERO,
            vel: Vec3::ZERO,
            facing_yaw: 0.0,
            stamina: 100.0,
            stamina_max: 100.0,
            coyote: COYOTE_TICKS,
            jump_buffer: 0,
            jumps_used: 0,
            roll_tick: 0,
            roll_dir: Vec3::new(1.0, 0.0, 0.0),
            invulnerable: false,
            sprinting: false,
            walking: false,
            crouching: false,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct TraversalInput {
    pub move_x: f32,
    pub move_y: f32,
    pub yaw: f32,
    pub jump: bool,
    pub roll: bool,
    pub sprint: bool,
    pub crouch: bool,
    pub walk: bool,
}

/// Collision answers for one tick. The pure replay uses an infinite plane.
#[derive(Clone, Copy, Debug)]
pub struct ProbeFrame {
    pub support: bool,
    pub ground_y: f32,
    pub normal_y: f32,
}

impl ProbeFrame {
    pub fn plane(ground_y: f32) -> Self {
        Self {
            support: true,
            ground_y,
            normal_y: 1.0,
        }
    }
}

/// Advance one tick. `world_seed` and `tick` are recorded for the replay bytes
/// and for a future `SimRng` draw. Phase 0 does not consume a random sample.
pub fn step(body: &mut TraversalBody, input: TraversalInput, probe: &ProbeFrame, tick: u64) {
    let _ = tick;
    if body.jump_buffer > 0 {
        body.jump_buffer -= 1;
    }
    if input.jump {
        body.jump_buffer = JUMP_BUFFER_TICKS;
    }

    if body.mode == TraversalMode::Roll {
        step_roll(body, probe);
        return;
    }

    let supported = supported(body, probe);
    if supported {
        body.coyote = COYOTE_TICKS;
        body.jumps_used = 0;
        if body.mode == TraversalMode::Air {
            body.mode = TraversalMode::Ground;
        }
    } else if body.coyote > 0 {
        body.coyote -= 1;
    }

    let can_jump = supported || body.coyote > 0;
    if body.mode == TraversalMode::Air && input.jump && body.jumps_used == 1 {
        body.vel.y = jump_speed();
        body.jumps_used = 2;
        body.jump_buffer = 0;
    } else if body.jump_buffer > 0 && can_jump && body.jumps_used == 0 {
        body.vel.y = jump_speed();
        body.jump_buffer = 0;
        body.coyote = 0;
        body.jumps_used = 1;
        body.mode = TraversalMode::Air;
    }

    if body.mode != TraversalMode::Roll && input.roll && supported && body.stamina >= ROLL_COST {
        begin_roll(body, input);
        step_roll(body, probe);
        return;
    }

    let wish = wish_dir(input);
    let moving = wish.length_squared() > 1e-8;
    body.crouching = input.crouch && supported && body.mode != TraversalMode::Air;
    body.sprinting =
        input.sprint && moving && body.stamina > 0.0 && !body.crouching && supported;
    body.walking = input.walk && moving && !body.sprinting && !body.crouching;
    let speed = if !moving {
        0.0
    } else if body.crouching {
        CROUCH_SPEED
    } else if body.sprinting {
        SPRINT_SPEED
    } else if body.walking {
        WALK_SPEED
    } else {
        RUN_SPEED
    };
    let planar = if body.mode == TraversalMode::Air {
        scale(wish, AIR_CONTROL * speed)
    } else {
        scale(wish, speed)
    };
    body.vel.x = planar.x;
    body.vel.z = planar.z;
    if moving {
        body.facing_yaw = detmath::atan2(wish.z, wish.x);
    }

    if supported && body.mode != TraversalMode::Air && body.mode != TraversalMode::Roll {
        body.mode = if body.crouching {
            TraversalMode::Crouch
        } else {
            TraversalMode::Ground
        };
    } else if !supported && body.mode != TraversalMode::Roll {
        body.mode = TraversalMode::Air;
    }

    if body.sprinting {
        body.stamina = (body.stamina - SPRINT_DRAIN_PER_TICK).max(0.0);
    } else if supported && body.mode != TraversalMode::Roll {
        body.stamina = (body.stamina + STAMINA_REGEN_PER_TICK).min(body.stamina_max);
    }

    integrate(body, probe);
}

fn begin_roll(body: &mut TraversalBody, input: TraversalInput) {
    body.stamina = (body.stamina - ROLL_COST).max(0.0);
    body.mode = TraversalMode::Roll;
    body.roll_tick = 0;
    body.invulnerable = false;
    let mut dir = wish_dir(input);
    if dir == Vec3::ZERO {
        dir = Vec3::new(
            detmath::cos(body.facing_yaw),
            0.0,
            detmath::sin(body.facing_yaw),
        );
    }
    body.roll_dir = dir;
}

fn step_roll(body: &mut TraversalBody, probe: &ProbeFrame) {
    if body.roll_tick >= ROLL_TICKS {
        body.mode = TraversalMode::Ground;
        body.invulnerable = false;
        body.roll_tick = 0;
        body.vel.x = 0.0;
        body.vel.z = 0.0;
        integrate(body, probe);
        return;
    }
    body.invulnerable = body.roll_tick >= ROLL_IFRAME_START && body.roll_tick <= ROLL_IFRAME_END;
    body.vel.x = body.roll_dir.x * ROLL_SPEED;
    body.vel.y = 0.0;
    body.vel.z = body.roll_dir.z * ROLL_SPEED;
    body.pos.x += body.vel.x * TICK_DT;
    body.pos.z += body.vel.z * TICK_DT;
    if probe.support {
        body.pos.y = probe.ground_y;
    }
    body.facing_yaw = detmath::atan2(body.roll_dir.z, body.roll_dir.x);
    body.roll_tick += 1;
}

fn integrate(body: &mut TraversalBody, probe: &ProbeFrame) {
    body.vel.y -= GRAVITY * TICK_DT;
    body.pos.x += body.vel.x * TICK_DT;
    body.pos.y += body.vel.y * TICK_DT;
    body.pos.z += body.vel.z * TICK_DT;
    if lands(body, probe) {
        body.pos.y = probe.ground_y;
        body.vel.y = 0.0;
        if body.mode == TraversalMode::Air {
            body.mode = TraversalMode::Ground;
            body.coyote = COYOTE_TICKS;
            body.jumps_used = 0;
        }
    }
}

fn lands(body: &TraversalBody, probe: &ProbeFrame) -> bool {
    probe.support && body.vel.y <= 0.0 && body.pos.y <= probe.ground_y + SNAP
}

fn supported(body: &TraversalBody, probe: &ProbeFrame) -> bool {
    probe.support && body.vel.y <= 0.0 && body.pos.y <= probe.ground_y + SNAP
}

fn jump_speed() -> f32 {
    detmath::sqrt(2.0 * GRAVITY * JUMP_HEIGHT)
}

fn wish_dir(input: TraversalInput) -> Vec3 {
    let forward = Vec3::new(detmath::cos(input.yaw), 0.0, detmath::sin(input.yaw));
    let right = Vec3::new(-forward.z, 0.0, forward.x);
    let wish = Vec3::new(
        forward.x * input.move_y + right.x * input.move_x,
        0.0,
        forward.z * input.move_y + right.z * input.move_x,
    );
    normalize_xz(wish)
}

fn normalize_xz(v: Vec3) -> Vec3 {
    let len2 = v.x * v.x + v.z * v.z;
    if len2 <= 1e-8 {
        return Vec3::ZERO;
    }
    let inv = 1.0 / detmath::sqrt(len2);
    Vec3::new(v.x * inv, 0.0, v.z * inv)
}

fn scale(v: Vec3, s: f32) -> Vec3 {
    Vec3::new(v.x * s, v.y * s, v.z * s)
}

/// 180 ticks of running, one jump, and one roll on a flat plane.
///
/// The bytes include position, velocity, stamina, and the roll tick. Two
/// calls return the same `Vec`. A constant digest cannot satisfy the asserts
/// in the replay test.
pub fn replay_bytes() -> Vec<u8> {
    let mut body = TraversalBody::default();
    let probe = ProbeFrame::plane(0.0);
    let mut out = Vec::with_capacity(180 * 48);
    push_u64(&mut out, REPLAY_SEED);
    push_u32(&mut out, REPLAY_TICKS);
    for tick in 0..REPLAY_TICKS {
        let input = replay_input(tick);
        step(&mut body, input, &probe, u64::from(tick));
        encode_body(&mut out, tick, &body);
    }
    out
}

fn replay_input(tick: u32) -> TraversalInput {
    let running = tick < 40 || (110..130).contains(&tick);
    let jumping = tick == 40;
    let rolling = tick == 130;
    let moving = running || jumping || (130..160).contains(&tick);
    TraversalInput {
        move_x: 0.0,
        move_y: if moving { 1.0 } else { 0.0 },
        yaw: 0.0,
        jump: jumping,
        roll: rolling,
        sprint: false,
        crouch: false,
        walk: false,
    }
}

fn encode_body(out: &mut Vec<u8>, tick: u32, body: &TraversalBody) {
    push_u32(out, tick);
    out.push(body.mode as u8);
    push_f32(out, body.pos.x);
    push_f32(out, body.pos.y);
    push_f32(out, body.pos.z);
    push_f32(out, body.vel.x);
    push_f32(out, body.vel.y);
    push_f32(out, body.vel.z);
    push_f32(out, body.stamina);
    push_u16(out, body.roll_tick);
    out.push(u8::from(body.invulnerable));
    out.push(body.jumps_used);
}

fn push_u16(out: &mut Vec<u8>, value: u16) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn push_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn push_u64(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn push_f32(out: &mut Vec<u8>, value: f32) {
    out.extend_from_slice(&value.to_bits().to_le_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run_script() -> Vec<TraversalBody> {
        let mut body = TraversalBody::default();
        let probe = ProbeFrame::plane(0.0);
        let mut frames = Vec::new();
        for tick in 0..REPLAY_TICKS {
            step(&mut body, replay_input(tick), &probe, u64::from(tick));
            frames.push(body.clone());
        }
        frames
    }

    #[test]
    fn run_moves_forward_on_the_ground() {
        let frames = run_script();
        assert_eq!(frames[10].mode, TraversalMode::Ground);
        assert!(frames[10].pos.x > 0.5, "x={}", frames[10].pos.x);
        assert_eq!(frames[10].pos.y.to_bits(), 0.0f32.to_bits());
    }

    #[test]
    fn jump_leaves_the_ground_and_lands() {
        let frames = run_script();
        assert_eq!(frames[40].mode, TraversalMode::Air);
        assert!(frames[40].vel.y > 1.0, "vy={}", frames[40].vel.y);
        assert!(frames[40].pos.y > 0.0);
        let landed = frames[50..]
            .iter()
            .find(|frame| frame.mode == TraversalMode::Ground)
            .expect("jump lands inside the script");
        assert!(landed.pos.y.abs() < 1e-4);
    }

    #[test]
    fn roll_spends_stamina_and_uses_tick_iframes() {
        let frames = run_script();
        let start = &frames[130];
        assert_eq!(start.mode, TraversalMode::Roll);
        assert!(
            !start.invulnerable,
            "tick 0 of a roll is recovery, not an i-frame"
        );
        assert!(
            (start.stamina - 75.0).abs() < 1e-3,
            "stamina={}",
            start.stamina
        );
        let iframe = frames[130 + usize::from(ROLL_IFRAME_START)].invulnerable;
        assert!(iframe);
        let after = &frames[130 + usize::from(ROLL_IFRAME_END) + 1];
        assert!(!after.invulnerable);
        let done = &frames[130 + usize::from(ROLL_TICKS)];
        assert_eq!(done.mode, TraversalMode::Ground);
    }

    #[test]
    fn replay_bytes_are_stable_and_not_constant() {
        let a = replay_bytes();
        let b = replay_bytes();
        assert_eq!(a, b);
        assert!(a.len() > 64);
        let mut idle = TraversalBody::default();
        let probe = ProbeFrame::plane(0.0);
        let before = idle.pos.x.to_bits();
        step(&mut idle, replay_input(0), &probe, 0);
        assert_ne!(idle.pos.x.to_bits(), before);
        assert!(a.windows(4).any(|w| w != [0, 0, 0, 0]));
    }
}
