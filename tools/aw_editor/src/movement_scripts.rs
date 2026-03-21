//! Movement scripts: predefined movement behaviors assignable to entities.
//!
//! Each script type defines how an entity moves over time when the editor
//! is in Play mode. Scripts are stored as JSON components on `EditorEntity`
//! and ticked by the runtime simulation loop.

use glam::{Quat, Vec3};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// The types of movement scripts available to entities.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum MovementScriptType {
    /// Patrol between a list of waypoints in order, then reverse or loop.
    Patrol,
    /// Follow another entity at a fixed distance.
    Follow,
    /// Orbit around a center point.
    Orbit,
    /// Move in a straight line at constant velocity.
    Linear,
    /// Wander randomly within a radius.
    Wander,
    /// Idle — no movement (placeholder for custom scripts).
    Idle,
}

impl std::fmt::Display for MovementScriptType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Patrol => write!(f, "Patrol"),
            Self::Follow => write!(f, "Follow"),
            Self::Orbit => write!(f, "Orbit"),
            Self::Linear => write!(f, "Linear"),
            Self::Wander => write!(f, "Wander"),
            Self::Idle => write!(f, "Idle"),
        }
    }
}

/// All available movement script types for the UI dropdown.
pub const ALL_SCRIPT_TYPES: &[MovementScriptType] = &[
    MovementScriptType::Patrol,
    MovementScriptType::Follow,
    MovementScriptType::Orbit,
    MovementScriptType::Linear,
    MovementScriptType::Wander,
    MovementScriptType::Idle,
];

/// Configuration for a movement script attached to an entity.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MovementScript {
    /// The type of movement behavior.
    pub script_type: MovementScriptType,
    /// Movement speed in units per second.
    pub speed: f32,
    /// Waypoints for Patrol mode (world-space positions).
    pub waypoints: Vec<Vec3>,
    /// Target entity ID for Follow mode.
    pub follow_target: Option<u64>,
    /// Follow distance (minimum distance to maintain).
    pub follow_distance: f32,
    /// Orbit radius for Orbit mode.
    pub orbit_radius: f32,
    /// Orbit angular speed (radians per second) for Orbit mode.
    pub orbit_speed: f32,
    /// Orbit center point.
    pub orbit_center: Vec3,
    /// Direction vector for Linear mode.
    pub direction: Vec3,
    /// Wander radius for Wander mode.
    pub wander_radius: f32,
    /// Wander center point.
    pub wander_center: Vec3,
    /// Whether to rotate the entity to face its movement direction.
    pub face_movement_direction: bool,
}

impl Default for MovementScript {
    fn default() -> Self {
        Self {
            script_type: MovementScriptType::Idle,
            speed: 2.0,
            waypoints: Vec::new(),
            follow_target: None,
            follow_distance: 2.0,
            orbit_radius: 5.0,
            orbit_speed: 1.0,
            orbit_center: Vec3::ZERO,
            direction: Vec3::new(1.0, 0.0, 0.0),
            wander_radius: 10.0,
            wander_center: Vec3::ZERO,
            face_movement_direction: true,
        }
    }
}

impl MovementScript {
    /// Create a new Patrol script with the given waypoints.
    pub fn patrol(waypoints: Vec<Vec3>, speed: f32) -> Self {
        Self {
            script_type: MovementScriptType::Patrol,
            speed,
            waypoints,
            ..Default::default()
        }
    }

    /// Create a new Follow script targeting an entity.
    pub fn follow(target_id: u64, distance: f32, speed: f32) -> Self {
        Self {
            script_type: MovementScriptType::Follow,
            speed,
            follow_target: Some(target_id),
            follow_distance: distance,
            ..Default::default()
        }
    }

    /// Create a new Orbit script.
    pub fn orbit(center: Vec3, radius: f32, angular_speed: f32) -> Self {
        Self {
            script_type: MovementScriptType::Orbit,
            orbit_center: center,
            orbit_radius: radius,
            orbit_speed: angular_speed,
            ..Default::default()
        }
    }

    /// Create a new Linear movement script.
    pub fn linear(direction: Vec3, speed: f32) -> Self {
        Self {
            script_type: MovementScriptType::Linear,
            speed,
            direction: direction.normalize_or_zero(),
            ..Default::default()
        }
    }

    /// Create a new Wander script.
    pub fn wander(center: Vec3, radius: f32, speed: f32) -> Self {
        Self {
            script_type: MovementScriptType::Wander,
            speed,
            wander_center: center,
            wander_radius: radius,
            ..Default::default()
        }
    }

    /// Serialize to JSON (for storing in EditorEntity.components).
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or_default()
    }

    /// Deserialize from JSON component value.
    pub fn from_json(value: &serde_json::Value) -> Option<Self> {
        serde_json::from_value(value.clone()).ok()
    }
}

/// Runtime state for an actively executing movement script.
#[derive(Clone, Debug)]
pub struct MovementRuntimeState {
    /// Current waypoint index (for Patrol).
    pub current_waypoint: usize,
    /// Whether patrol is going forward (+1) or backward (-1).
    pub patrol_direction: i32,
    /// Accumulated orbit angle (for Orbit).
    pub orbit_angle: f32,
    /// Current wander target (for Wander).
    pub wander_target: Vec3,
    /// Time until next wander target is chosen.
    pub wander_cooldown: f32,
    /// Total elapsed time of this script execution.
    pub elapsed: f32,
}

impl Default for MovementRuntimeState {
    fn default() -> Self {
        Self {
            current_waypoint: 0,
            patrol_direction: 1,
            orbit_angle: 0.0,
            wander_target: Vec3::ZERO,
            wander_cooldown: 0.0,
            elapsed: 0.0,
        }
    }
}

/// The movement system that ticks all active scripts each frame.
pub struct MovementSystem {
    /// Per-entity runtime states.
    runtime_states: HashMap<u64, MovementRuntimeState>,
    /// Time spent in last tick (ms).
    pub last_tick_ms: f32,
    /// Simple RNG state for wander behavior.
    rng_state: u64,
}

impl Default for MovementSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl MovementSystem {
    pub fn new() -> Self {
        Self {
            runtime_states: HashMap::new(),
            last_tick_ms: 0.0,
            rng_state: 12345,
        }
    }

    /// Simple xorshift64 pseudo-random number generator.
    fn next_random(&mut self) -> f32 {
        self.rng_state ^= self.rng_state << 13;
        self.rng_state ^= self.rng_state >> 7;
        self.rng_state ^= self.rng_state << 17;
        // Map to [0, 1) range
        (self.rng_state as f32 / u64::MAX as f32).abs()
    }

    /// Tick a single entity's movement script. Returns (new_position, new_rotation).
    pub fn tick_entity(
        &mut self,
        entity_id: u64,
        script: &MovementScript,
        current_pos: Vec3,
        _current_rot: Quat,
        dt: f32,
        entity_positions: &HashMap<u64, Vec3>,
    ) -> (Vec3, Quat) {
        // Pre-generate random values before borrowing runtime_states
        // (avoids simultaneous mutable borrow of self through runtime_states + rng_state)
        let rand_angle = self.next_random() * std::f32::consts::TAU;
        let rand_dist = self.next_random();
        let rand_cooldown = 2.0 + self.next_random() * 3.0;

        let state = self
            .runtime_states
            .entry(entity_id)
            .or_insert_with(MovementRuntimeState::default);
        state.elapsed += dt;

        match script.script_type {
            MovementScriptType::Idle => (current_pos, Quat::IDENTITY),

            MovementScriptType::Linear => {
                let velocity = script.direction.normalize_or_zero() * script.speed;
                let new_pos = current_pos + velocity * dt;
                let rot = if script.face_movement_direction && velocity.length_squared() > 0.001 {
                    look_rotation(velocity.normalize())
                } else {
                    Quat::IDENTITY
                };
                (new_pos, rot)
            }

            MovementScriptType::Patrol => {
                if script.waypoints.is_empty() {
                    return (current_pos, Quat::IDENTITY);
                }
                let target = script.waypoints[state.current_waypoint % script.waypoints.len()];
                let to_target = target - current_pos;
                let dist = to_target.length();

                if dist < 0.1 {
                    // Reached waypoint, move to next
                    let next = state.current_waypoint as i32 + state.patrol_direction;
                    if next < 0 || next >= script.waypoints.len() as i32 {
                        state.patrol_direction = -state.patrol_direction;
                    }
                    state.current_waypoint =
                        ((state.current_waypoint as i32 + state.patrol_direction).max(0) as usize)
                            .min(script.waypoints.len().saturating_sub(1));
                    (current_pos, Quat::IDENTITY)
                } else {
                    let dir = to_target / dist;
                    let step = (script.speed * dt).min(dist);
                    let new_pos = current_pos + dir * step;
                    let rot = if script.face_movement_direction {
                        look_rotation(dir)
                    } else {
                        Quat::IDENTITY
                    };
                    (new_pos, rot)
                }
            }

            MovementScriptType::Follow => {
                if let Some(target_id) = script.follow_target {
                    if let Some(&target_pos) = entity_positions.get(&target_id) {
                        let to_target = target_pos - current_pos;
                        let dist = to_target.length();
                        if dist > script.follow_distance {
                            let dir = to_target / dist;
                            let step = (script.speed * dt).min(dist - script.follow_distance);
                            let new_pos = current_pos + dir * step;
                            let rot = if script.face_movement_direction {
                                look_rotation(dir)
                            } else {
                                Quat::IDENTITY
                            };
                            return (new_pos, rot);
                        }
                    }
                }
                (current_pos, Quat::IDENTITY)
            }

            MovementScriptType::Orbit => {
                state.orbit_angle += script.orbit_speed * dt;
                let x = script.orbit_center.x + script.orbit_radius * state.orbit_angle.cos();
                let z = script.orbit_center.z + script.orbit_radius * state.orbit_angle.sin();
                let new_pos = Vec3::new(x, script.orbit_center.y, z);
                let dir = (new_pos - current_pos).normalize_or_zero();
                let rot = if script.face_movement_direction && dir.length_squared() > 0.001 {
                    look_rotation(dir)
                } else {
                    Quat::IDENTITY
                };
                (new_pos, rot)
            }

            MovementScriptType::Wander => {
                state.wander_cooldown -= dt;
                if state.wander_cooldown <= 0.0 || state.wander_target == Vec3::ZERO {
                    // Use pre-generated random values
                    let dist = rand_dist * script.wander_radius;
                    state.wander_target = script.wander_center
                        + Vec3::new(rand_angle.cos() * dist, 0.0, rand_angle.sin() * dist);
                    state.wander_cooldown = rand_cooldown;
                }

                let to_target = state.wander_target - current_pos;
                let dist = to_target.length();
                if dist > 0.2 {
                    let dir = to_target / dist;
                    let step = (script.speed * dt).min(dist);
                    let new_pos = current_pos + dir * step;
                    let rot = if script.face_movement_direction {
                        look_rotation(dir)
                    } else {
                        Quat::IDENTITY
                    };
                    (new_pos, rot)
                } else {
                    (current_pos, Quat::IDENTITY)
                }
            }
        }
    }

    /// Tick all entities that have movement scripts.
    /// Takes a map of (entity_id → (script, current_position, current_rotation)).
    /// Returns a map of (entity_id → (new_position, new_rotation)).
    pub fn tick_all(
        &mut self,
        entities: &[(u64, MovementScript, Vec3, Quat)],
        dt: f32,
    ) -> Vec<(u64, Vec3, Quat)> {
        let start = std::time::Instant::now();

        // Build position lookup for Follow scripts
        let positions: HashMap<u64, Vec3> =
            entities.iter().map(|(id, _, pos, _)| (*id, *pos)).collect();

        let results: Vec<(u64, Vec3, Quat)> = entities
            .iter()
            .map(|(id, script, pos, rot)| {
                let (new_pos, new_rot) = self.tick_entity(*id, script, *pos, *rot, dt, &positions);
                (*id, new_pos, new_rot)
            })
            .collect();

        self.last_tick_ms = start.elapsed().as_secs_f32() * 1000.0;
        results
    }

    /// Remove runtime state for a deleted entity.
    pub fn remove_entity(&mut self, entity_id: u64) {
        self.runtime_states.remove(&entity_id);
    }

    /// Reset all runtime states (e.g., when entering play mode).
    pub fn reset(&mut self) {
        self.runtime_states.clear();
    }
}

/// Compute a rotation quaternion that looks along `forward` (Y-up world).
fn look_rotation(forward: Vec3) -> Quat {
    let forward = Vec3::new(forward.x, 0.0, forward.z).normalize_or_zero();
    if forward.length_squared() < 0.001 {
        return Quat::IDENTITY;
    }
    Quat::from_rotation_arc(Vec3::NEG_Z, forward)
}
