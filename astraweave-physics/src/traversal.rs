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
    Vault = 5,
    Mantle = 6,
    Climb = 7,
    Swim = 8,
    Dive = 9,
    Slide = 10,
    Prone = 11,
    WallRun = 12,
    Glide = 13,
    LedgeHang = 14,
    Shimmy = 15,
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
    pub action_tick: u16,
    pub action_from: Vec3,
    pub action_to: Vec3,
    pub climb_normal: Vec3,
    pub slide_speed: f32,
    pub water_hold: u16,
    pub wall_tick: u16,
    /// 0 dry, 1 soaking. Multiplies climb drain. Comes from `WaterPlayerState`.
    pub wetness: f32,
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
            action_tick: 0,
            action_from: Vec3::ZERO,
            action_to: Vec3::ZERO,
            climb_normal: Vec3::Y,
            slide_speed: 0.0,
            water_hold: 0,
            wall_tick: 0,
            wetness: 0.0,
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
    pub climb: bool,
    pub glide: bool,
    pub dive: bool,
    pub prone: bool,
}

/// Collision answers for one tick. The pure replay uses an infinite plane.
pub const MAX_STEP: f32 = 0.4;
pub const VAULT_TICKS: u16 = 12;
pub const MANTLE_TICKS: u16 = 20;
pub const CLIMB_SPEED: f32 = 1.6;
pub const CLIMB_DRAIN_PER_TICK: f32 = 10.0 * TICK_DT;
pub const OVERHANG_MIN_NORMAL_Y: f32 = -0.35;
pub const SWIM_DEPTH: f32 = 1.1;
pub const DIVE_DEPTH: f32 = 1.7;
pub const WATER_HYSTERESIS: u16 = 2;
pub const GLIDE_SINK: f32 = 1.5;
pub const GLIDE_DRAIN_PER_TICK: f32 = 8.0 * TICK_DT;
pub const SLIDE_FRICTION: f32 = 0.4;
pub const WALLRUN_TICKS: u16 = 40;
pub const WALLRUN_SPEED: f32 = 7.0;
pub const COLLISION_DEVIATION_BUDGET: f32 = 0.05;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Climbability {
    Climbable,
    Slippery,
    NonClimbable,
}

#[derive(Clone, Copy, Debug)]
pub struct ProbeFrame {
    pub support: bool,
    pub ground_y: f32,
    pub normal_y: f32,
    pub step_height: Option<f32>,
    pub vault_to: Option<Vec3>,
    pub mantle_to: Option<Vec3>,
    pub climb_point: Option<Vec3>,
    pub climb_normal: Option<Vec3>,
    pub markup_point: Option<Vec3>,
    pub climbability: Climbability,
    pub water_surface: Option<f32>,
    pub ceiling_blocked: bool,
    pub wall_normal: Option<Vec3>,
    pub updraft: bool,
    pub ledge_point: Option<Vec3>,
}

impl ProbeFrame {
    pub fn plane(ground_y: f32) -> Self {
        Self {
            support: true,
            ground_y,
            normal_y: 1.0,
            step_height: None,
            vault_to: None,
            mantle_to: None,
            climb_point: None,
            climb_normal: None,
            markup_point: None,
            climbability: Climbability::Climbable,
            water_surface: None,
            ceiling_blocked: false,
            wall_normal: None,
            updraft: false,
            ledge_point: None,
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
    if is_special(body.mode) {
        step_special(body, input, probe);
        return;
    }
    if try_begin_special(body, input, probe) {
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
    body.sprinting = input.sprint && moving && body.stamina > 0.0 && !body.crouching && supported;
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

fn is_special(mode: TraversalMode) -> bool {
    matches!(
        mode,
        TraversalMode::Vault
            | TraversalMode::Mantle
            | TraversalMode::Climb
            | TraversalMode::Swim
            | TraversalMode::Dive
            | TraversalMode::Slide
            | TraversalMode::Prone
            | TraversalMode::WallRun
            | TraversalMode::Glide
            | TraversalMode::LedgeHang
            | TraversalMode::Shimmy
    )
}

fn try_begin_special(body: &mut TraversalBody, input: TraversalInput, probe: &ProbeFrame) -> bool {
    let moving = input.move_x.abs() + input.move_y.abs() > 0.1;
    if let Some(to) = probe.mantle_to {
        if moving && supported(body, probe) {
            begin_arc(body, TraversalMode::Mantle, to);
            return true;
        }
    }
    if let Some(to) = probe.vault_to {
        if moving {
            begin_arc(body, TraversalMode::Vault, to);
            return true;
        }
    }
    if (input.climb || input.jump) && begin_climb(body, probe) {
        return true;
    }
    if let Some(surface) = probe.water_surface {
        let depth = surface - body.pos.y;
        if depth >= SWIM_DEPTH {
            body.water_hold = body.water_hold.saturating_add(1);
        } else if body.water_hold > 0 {
            body.water_hold -= 1;
        }
        if body.water_hold >= WATER_HYSTERESIS && depth >= SWIM_DEPTH {
            body.mode = if input.dive || depth >= DIVE_DEPTH {
                TraversalMode::Dive
            } else {
                TraversalMode::Swim
            };
            body.vel = Vec3::ZERO;
            return true;
        }
    }
    if input.glide && body.mode == TraversalMode::Air && body.stamina > 0.0 {
        body.mode = TraversalMode::Glide;
        step_glide(body, input, probe);
        return true;
    }
    if input.prone && supported(body, probe) && !probe.ceiling_blocked {
        body.mode = TraversalMode::Prone;
        return true;
    }
    if input.crouch && input.sprint && supported(body, probe) && moving {
        body.mode = TraversalMode::Slide;
        body.slide_speed = SPRINT_SPEED;
        return true;
    }
    if probe.normal_y < 0.5 && supported(body, probe) && moving {
        body.mode = TraversalMode::Slide;
        body.slide_speed = RUN_SPEED;
        return true;
    }
    if let Some(normal) = probe.wall_normal {
        let speed2 = body.vel.x * body.vel.x + body.vel.z * body.vel.z;
        if body.mode == TraversalMode::Air && speed2 > 16.0 {
            body.mode = TraversalMode::WallRun;
            body.climb_normal = normal;
            body.wall_tick = 0;
            return true;
        }
    }
    false
}

fn begin_arc(body: &mut TraversalBody, mode: TraversalMode, to: Vec3) {
    body.action_from = body.pos;
    body.action_to = to;
    body.action_tick = 0;
    body.mode = mode;
    body.vel = Vec3::ZERO;
}

fn begin_climb(body: &mut TraversalBody, probe: &ProbeFrame) -> bool {
    if probe.climbability == Climbability::NonClimbable || body.stamina <= 0.0 {
        return false;
    }
    let (point, normal) = if let Some(point) = probe.markup_point {
        (point, probe.climb_normal.unwrap_or(Vec3::Y))
    } else if let Some(point) = probe.climb_point {
        (
            point,
            probe.climb_normal.unwrap_or(Vec3::new(0.0, 0.0, 1.0)),
        )
    } else {
        return false;
    };
    if normal.y < OVERHANG_MIN_NORMAL_Y {
        return false;
    }
    body.mode = TraversalMode::Climb;
    body.action_to = point;
    body.climb_normal = normal;
    body.pos = point;
    body.vel = Vec3::ZERO;
    true
}

fn step_special(body: &mut TraversalBody, input: TraversalInput, probe: &ProbeFrame) {
    match body.mode {
        TraversalMode::Vault => step_arc(body, VAULT_TICKS, 0.35),
        TraversalMode::Mantle => step_arc(body, MANTLE_TICKS, 0.55),
        TraversalMode::Climb => step_climb(body, input, probe),
        TraversalMode::LedgeHang => step_ledge(body, input, probe),
        TraversalMode::Shimmy => step_shimmy(body, input, probe),
        TraversalMode::Swim => step_swim(body, input, probe, false),
        TraversalMode::Dive => step_swim(body, input, probe, true),
        TraversalMode::Slide => step_slide(body, input, probe),
        TraversalMode::Prone => step_prone(body, input, probe),
        TraversalMode::WallRun => step_wallrun(body, input, probe),
        TraversalMode::Glide => step_glide(body, input, probe),
        _ => {}
    }
}

fn step_arc(body: &mut TraversalBody, duration: u16, arch: f32) {
    body.action_tick = body.action_tick.saturating_add(1);
    let t = (f32::from(body.action_tick) / f32::from(duration)).min(1.0);
    let base = body.action_from + (body.action_to - body.action_from) * t;
    let bump = detmath::sin(t * std::f32::consts::PI) * arch;
    body.pos = Vec3::new(base.x, base.y + bump, base.z);
    if body.action_tick >= duration {
        body.pos = body.action_to;
        body.vel = Vec3::ZERO;
        body.mode = TraversalMode::Ground;
        body.action_tick = 0;
    }
}

fn step_climb(body: &mut TraversalBody, input: TraversalInput, probe: &ProbeFrame) {
    if probe.climbability == Climbability::NonClimbable || body.stamina <= 0.0 {
        body.mode = TraversalMode::Air;
        body.vel = Vec3::ZERO;
        return;
    }
    let before = body.stamina;
    let mut drain = CLIMB_DRAIN_PER_TICK;
    if probe.climbability == Climbability::Slippery {
        drain *= 1.5;
    }
    drain *= 1.0 + body.wetness.clamp(0.0, 1.0);
    body.stamina = (body.stamina - drain).max(0.0);
    let _ = before;
    let wish = wish_dir(input);
    let mut delta = scale(wish, CLIMB_SPEED * TICK_DT);
    if probe.climbability == Climbability::Slippery {
        delta.y -= 0.4 * TICK_DT;
    }
    body.pos += delta;
    if let Some(normal) = probe.climb_normal {
        body.climb_normal = normal;
        if normal.y < OVERHANG_MIN_NORMAL_Y {
            body.mode = TraversalMode::Air;
            return;
        }
    }
    if let Some(ledge) = probe.ledge_point {
        body.mode = TraversalMode::LedgeHang;
        body.pos = ledge;
        body.vel = Vec3::ZERO;
    }
    if body.stamina <= 0.0 {
        body.mode = TraversalMode::Air;
    }
}

fn step_ledge(body: &mut TraversalBody, input: TraversalInput, probe: &ProbeFrame) {
    if probe.ledge_point.is_none() {
        body.mode = TraversalMode::Air;
        return;
    }
    body.pos = probe.ledge_point.unwrap_or(body.pos);
    if input.move_x.abs() > 0.1 {
        body.mode = TraversalMode::Shimmy;
    }
}

fn step_shimmy(body: &mut TraversalBody, input: TraversalInput, probe: &ProbeFrame) {
    if probe.ledge_point.is_none() {
        body.mode = TraversalMode::Air;
        return;
    }
    body.pos.x += input.move_x * 1.2 * TICK_DT;
    body.pos.y = probe.ledge_point.unwrap_or(body.pos).y;
    if input.move_x.abs() <= 0.1 {
        body.mode = TraversalMode::LedgeHang;
    }
}

fn step_swim(body: &mut TraversalBody, input: TraversalInput, probe: &ProbeFrame, diving: bool) {
    let surface = probe.water_surface.unwrap_or(body.pos.y);
    let depth = surface - body.pos.y;
    if depth < SWIM_DEPTH - 0.2 && body.water_hold == 0 {
        body.mode = TraversalMode::Ground;
        return;
    }
    if depth < SWIM_DEPTH {
        if body.water_hold > 0 {
            body.water_hold -= 1;
        }
        if body.water_hold == 0 && depth < SWIM_DEPTH - 0.05 {
            body.mode = TraversalMode::Ground;
            return;
        }
    }
    let wish = wish_dir(input);
    let speed = if diving { 2.2 } else { 2.8 };
    body.vel.x = wish.x * speed;
    body.vel.z = wish.z * speed;
    body.pos.x += body.vel.x * TICK_DT;
    body.pos.z += body.vel.z * TICK_DT;
    if diving || input.dive || depth >= DIVE_DEPTH {
        body.mode = TraversalMode::Dive;
        let vertical = if input.jump { 1.5 } else { -1.2 };
        body.vel.y = vertical;
        body.pos.y += body.vel.y * TICK_DT;
    } else {
        body.mode = TraversalMode::Swim;
        body.pos.y = surface - 0.2;
        body.vel.y = 0.0;
    }
}

fn step_slide(body: &mut TraversalBody, input: TraversalInput, probe: &ProbeFrame) {
    if input.jump {
        body.mode = TraversalMode::Air;
        body.vel.x = detmath::cos(body.facing_yaw) * body.slide_speed;
        body.vel.z = detmath::sin(body.facing_yaw) * body.slide_speed;
        body.vel.y = jump_speed() * 0.5;
        return;
    }
    let downhill = if probe.normal_y < 0.85 { 2.0 } else { 0.0 };
    body.slide_speed = (body.slide_speed + downhill * TICK_DT - SLIDE_FRICTION * TICK_DT).max(0.0);
    body.pos.x += detmath::cos(body.facing_yaw) * body.slide_speed * TICK_DT;
    body.pos.z += detmath::sin(body.facing_yaw) * body.slide_speed * TICK_DT;
    if probe.support {
        body.pos.y = floor_y(probe);
    }
    if body.slide_speed < 0.5 && probe.normal_y >= 0.85 {
        body.mode = TraversalMode::Ground;
        body.vel = Vec3::ZERO;
    }
}

fn step_prone(body: &mut TraversalBody, input: TraversalInput, probe: &ProbeFrame) {
    if !input.prone && !probe.ceiling_blocked {
        body.mode = TraversalMode::Crouch;
        return;
    }
    let wish = wish_dir(input);
    body.vel.x = wish.x * 0.8;
    body.vel.z = wish.z * 0.8;
    body.pos.x += body.vel.x * TICK_DT;
    body.pos.z += body.vel.z * TICK_DT;
    if probe.support {
        body.pos.y = floor_y(probe);
    }
}

fn step_wallrun(body: &mut TraversalBody, input: TraversalInput, probe: &ProbeFrame) {
    body.wall_tick = body.wall_tick.saturating_add(1);
    if probe.wall_normal.is_none() || body.wall_tick > WALLRUN_TICKS {
        body.mode = TraversalMode::Air;
        return;
    }
    let wish = wish_dir(input);
    let along = if wish.length_squared() <= 1e-8 {
        Vec3::new(-body.climb_normal.z, 0.0, body.climb_normal.x)
    } else {
        wish
    };
    body.vel.x = along.x * WALLRUN_SPEED;
    body.vel.z = along.z * WALLRUN_SPEED;
    body.vel.y = -1.0;
    body.pos.x += body.vel.x * TICK_DT;
    body.pos.y += body.vel.y * TICK_DT;
    body.pos.z += body.vel.z * TICK_DT;
}

fn step_glide(body: &mut TraversalBody, input: TraversalInput, probe: &ProbeFrame) {
    if !input.glide || body.stamina <= 0.0 {
        body.mode = TraversalMode::Air;
        return;
    }
    body.stamina = (body.stamina - GLIDE_DRAIN_PER_TICK).max(0.0);
    let wish = wish_dir(input);
    body.vel.x = wish.x * 6.0;
    body.vel.z = wish.z * 6.0;
    body.vel.y = if probe.updraft { 2.0 } else { -GLIDE_SINK };
    body.pos += body.vel * TICK_DT;
    if body.stamina <= 0.0 {
        body.mode = TraversalMode::Air;
    }
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
        body.pos.y = floor_y(probe);
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
        body.pos.y = floor_y(probe);
        body.vel.y = 0.0;
        if body.mode == TraversalMode::Air {
            body.mode = TraversalMode::Ground;
            body.coyote = COYOTE_TICKS;
            body.jumps_used = 0;
        }
    }
}

fn floor_y(probe: &ProbeFrame) -> f32 {
    match probe.step_height {
        Some(height) if height > 0.0 && height <= MAX_STEP => probe.ground_y + height,
        _ => probe.ground_y,
    }
}

fn lands(body: &TraversalBody, probe: &ProbeFrame) -> bool {
    probe.support && body.vel.y <= 0.0 && body.pos.y <= floor_y(probe) + SNAP
}

fn supported(body: &TraversalBody, probe: &ProbeFrame) -> bool {
    probe.support && body.vel.y <= 0.0 && body.pos.y <= floor_y(probe) + SNAP
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

/// Largest gap from a render-mesh sample to the nearest collider sample.
/// Climbable assets must stay within [`COLLISION_DEVIATION_BUDGET`].
pub fn max_collider_deviation(render: &[Vec3], collider: &[Vec3]) -> f32 {
    let mut max = 0.0f32;
    for point in render {
        let mut best = f32::MAX;
        for sample in collider {
            let dx = point.x - sample.x;
            let dy = point.y - sample.y;
            let dz = point.z - sample.z;
            let dist = detmath::sqrt(dx * dx + dy * dy + dz * dz);
            if dist < best {
                best = dist;
            }
        }
        if best > max {
            max = best;
        }
    }
    max
}

pub fn climb_collision_within_budget(render: &[Vec3], collider: &[Vec3]) -> bool {
    max_collider_deviation(render, collider) <= COLLISION_DEVIATION_BUDGET
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
        climb: false,
        glide: false,
        dive: false,
        prone: false,
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

    fn tick(body: &mut TraversalBody, input: TraversalInput, probe: &ProbeFrame) {
        step(body, input, probe, 0);
    }

    #[test]
    fn step_up_raises_the_feet_and_a_tall_riser_does_not() {
        let mut body = TraversalBody::default();
        let mut probe = ProbeFrame::plane(0.0);
        probe.step_height = Some(0.3);
        tick(
            &mut body,
            TraversalInput {
                move_y: 1.0,
                ..TraversalInput::default()
            },
            &probe,
        );
        assert!((body.pos.y - 0.3).abs() < 1e-3, "y={}", body.pos.y);
        let mut tall = TraversalBody::default();
        probe.step_height = Some(0.9);
        tick(
            &mut tall,
            TraversalInput {
                move_y: 1.0,
                ..TraversalInput::default()
            },
            &probe,
        );
        assert!(tall.pos.y.abs() < 1e-3, "tall riser y={}", tall.pos.y);
    }

    #[test]
    fn vault_and_mantle_finish_on_the_landing() {
        let landing = Vec3::new(1.2, 0.8, 0.0);
        let mut body = TraversalBody::default();
        let mut probe = ProbeFrame::plane(0.0);
        probe.vault_to = Some(landing);
        tick(
            &mut body,
            TraversalInput {
                move_y: 1.0,
                ..TraversalInput::default()
            },
            &probe,
        );
        assert_eq!(body.mode, TraversalMode::Vault);
        for _ in 0..usize::from(VAULT_TICKS) {
            tick(&mut body, TraversalInput::default(), &probe);
        }
        assert_eq!(body.mode, TraversalMode::Ground);
        assert!((body.pos.x - landing.x).abs() < 1e-3);
        assert!((body.pos.y - landing.y).abs() < 1e-3);

        let mut climber = TraversalBody::default();
        probe.vault_to = None;
        probe.mantle_to = Some(Vec3::new(0.4, 1.4, 0.0));
        tick(
            &mut climber,
            TraversalInput {
                move_y: 1.0,
                ..TraversalInput::default()
            },
            &probe,
        );
        assert_eq!(climber.mode, TraversalMode::Mantle);
    }

    #[test]
    fn hybrid_climb_prefers_markup_and_locks_regen() {
        let mut body = TraversalBody::default();
        body.stamina = 40.0;
        let mut probe = ProbeFrame::plane(0.0);
        probe.climb_point = Some(Vec3::new(0.0, 1.0, 0.2));
        probe.markup_point = Some(Vec3::new(0.0, 1.2, 0.0));
        probe.climb_normal = Some(Vec3::new(0.0, 0.2, 1.0));
        tick(
            &mut body,
            TraversalInput {
                climb: true,
                ..TraversalInput::default()
            },
            &probe,
        );
        assert_eq!(body.mode, TraversalMode::Climb);
        assert!((body.pos.y - 1.2).abs() < 1e-4, "markup should win");
        let start = body.stamina;
        tick(&mut body, TraversalInput::default(), &probe);
        assert!(body.stamina < start, "climb must drain");
        let hung = body.stamina;
        tick(&mut body, TraversalInput::default(), &probe);
        assert!(
            body.stamina <= hung,
            "regen is locked while climbing: {hung} -> {}",
            body.stamina
        );
    }

    #[test]
    fn nonclimbable_and_overhang_and_slippery_are_enforced() {
        let mut body = TraversalBody::default();
        let mut probe = ProbeFrame::plane(0.0);
        probe.climb_point = Some(Vec3::new(0.0, 2.0, 0.0));
        probe.climbability = Climbability::NonClimbable;
        tick(
            &mut body,
            TraversalInput {
                climb: true,
                ..TraversalInput::default()
            },
            &probe,
        );
        assert_ne!(body.mode, TraversalMode::Climb);

        probe.climbability = Climbability::Climbable;
        probe.climb_normal = Some(Vec3::new(0.0, -0.9, 0.1));
        tick(
            &mut body,
            TraversalInput {
                climb: true,
                ..TraversalInput::default()
            },
            &probe,
        );
        assert_ne!(body.mode, TraversalMode::Climb);

        probe.climb_normal = Some(Vec3::new(0.0, 0.0, 1.0));
        probe.climbability = Climbability::Slippery;
        tick(
            &mut body,
            TraversalInput {
                climb: true,
                crouch: true,
                ..TraversalInput::default()
            },
            &probe,
        );
        assert_eq!(body.mode, TraversalMode::Climb);
        let y = body.pos.y;
        tick(
            &mut body,
            TraversalInput {
                crouch: true,
                ..TraversalInput::default()
            },
            &probe,
        );
        assert!(body.pos.y < y, "slippery crouch still slides down");
    }

    #[test]
    fn swim_dive_slide_prone_wallrun_and_glide() {
        let mut body = TraversalBody::default();
        let mut probe = ProbeFrame::plane(0.0);
        probe.water_surface = Some(1.4);
        tick(&mut body, TraversalInput::default(), &probe);
        assert_ne!(body.mode, TraversalMode::Swim);
        tick(&mut body, TraversalInput::default(), &probe);
        assert_eq!(body.mode, TraversalMode::Swim);
        tick(
            &mut body,
            TraversalInput {
                dive: true,
                ..TraversalInput::default()
            },
            &probe,
        );
        assert_eq!(body.mode, TraversalMode::Dive);

        let mut slider = TraversalBody::default();
        slider.facing_yaw = 0.0;
        let ground = ProbeFrame::plane(0.0);
        tick(
            &mut slider,
            TraversalInput {
                move_y: 1.0,
                crouch: true,
                sprint: true,
                ..TraversalInput::default()
            },
            &ground,
        );
        assert_eq!(slider.mode, TraversalMode::Slide);

        let mut prone = TraversalBody::default();
        tick(
            &mut prone,
            TraversalInput {
                prone: true,
                ..TraversalInput::default()
            },
            &ground,
        );
        assert_eq!(prone.mode, TraversalMode::Prone);
        let mut ceiling = ground;
        ceiling.ceiling_blocked = true;
        tick(&mut prone, TraversalInput::default(), &ceiling);
        assert_eq!(prone.mode, TraversalMode::Prone);

        let mut runner = TraversalBody::default();
        runner.mode = TraversalMode::Air;
        runner.vel = Vec3::new(6.0, 0.0, 0.0);
        runner.pos.y = 1.0;
        let mut wall = ProbeFrame::plane(0.0);
        wall.support = false;
        wall.wall_normal = Some(Vec3::new(0.0, 0.0, 1.0));
        tick(
            &mut runner,
            TraversalInput {
                move_y: 1.0,
                ..TraversalInput::default()
            },
            &wall,
        );
        assert_eq!(runner.mode, TraversalMode::WallRun);

        let mut glider = TraversalBody::default();
        glider.mode = TraversalMode::Air;
        glider.pos.y = 8.0;
        let mut air = ProbeFrame::plane(0.0);
        air.support = false;
        air.updraft = true;
        tick(
            &mut glider,
            TraversalInput {
                glide: true,
                move_y: 1.0,
                ..TraversalInput::default()
            },
            &air,
        );
        assert_eq!(glider.mode, TraversalMode::Glide);
        assert!(glider.vel.y > 0.0, "updraft lifts");
    }

    #[test]
    fn collision_lint_rejects_a_loose_hull() {
        let render = [Vec3::new(0.0, 1.0, 0.0), Vec3::new(0.02, 1.0, 0.0)];
        let tight = [Vec3::new(0.0, 1.0, 0.0), Vec3::new(0.02, 1.01, 0.0)];
        let loose = [Vec3::new(0.0, 1.2, 0.0)];
        assert!(climb_collision_within_budget(&render, &tight));
        assert!(!climb_collision_within_budget(&render, &loose));
    }
}
