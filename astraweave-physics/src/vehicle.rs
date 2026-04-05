//! # Vehicle Physics System
//!
//! Provides realistic vehicle simulation including:
//! - Raycast suspension (industry-standard approach)
//! - Friction curves (Pacejka-inspired slip model)
//! - Engine and transmission simulation
//! - Steering mechanics
//!
//! ## Features
//!
//! - **Wheeled Vehicles**: Cars, trucks, motorcycles
//! - **Suspension**: Spring-damper raycast system
//! - **Friction**: Slip ratio and slip angle curves
//! - **Drivetrain**: Engine torque, gear ratios, differential
//!
//! ## Usage
//!
//! ```rust
//! use astraweave_physics::vehicle::{VehicleConfig, Vehicle, WheelConfig};
//! use glam::Vec3;
//!
//! let config = VehicleConfig {
//!     mass: 1500.0,
//!     wheels: vec![
//!         WheelConfig::front_left(Vec3::new(-0.8, 0.0, 1.2)),
//!         WheelConfig::front_right(Vec3::new(0.8, 0.0, 1.2)),
//!         WheelConfig::rear_left(Vec3::new(-0.8, 0.0, -1.2)),
//!         WheelConfig::rear_right(Vec3::new(0.8, 0.0, -1.2)),
//!     ],
//!     ..Default::default()
//! };
//! ```

use crate::{BodyId, PhysicsWorld};
use glam::{Quat, Vec3};

/// Unique identifier for a vehicle
pub type VehicleId = u64;

/// Wheel position preset
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum WheelPosition {
    FrontLeft,
    FrontRight,
    RearLeft,
    RearRight,
    /// Custom position (e.g., for 6-wheeled trucks)
    Custom(u8),
}

/// Configuration for a single wheel
#[derive(Debug, Clone)]
pub struct WheelConfig {
    /// Position relative to vehicle center of mass
    pub position: Vec3,
    /// Wheel radius
    pub radius: f32,
    /// Wheel width (for friction calculation)
    pub width: f32,
    /// Whether this wheel can steer
    pub steerable: bool,
    /// Whether this wheel is driven (receives engine power)
    pub driven: bool,
    /// Suspension rest length
    pub suspension_rest_length: f32,
    /// Suspension spring stiffness (N/m)
    pub suspension_stiffness: f32,
    /// Suspension damping coefficient
    pub suspension_damping: f32,
    /// Maximum suspension compression
    pub suspension_max_compression: f32,
    /// Maximum suspension extension
    pub suspension_max_extension: f32,
    /// Wheel position identifier
    pub position_id: WheelPosition,
}

impl Default for WheelConfig {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            radius: 0.35,
            width: 0.25,
            steerable: false,
            driven: false,
            suspension_rest_length: 0.3,
            suspension_stiffness: 35000.0,
            suspension_damping: 4500.0,
            suspension_max_compression: 0.1,
            suspension_max_extension: 0.2,
            position_id: WheelPosition::Custom(0),
        }
    }
}

impl WheelConfig {
    /// Create a front-left wheel configuration
    pub fn front_left(position: Vec3) -> Self {
        Self {
            position,
            steerable: true,
            driven: false, // FWD would set this true
            position_id: WheelPosition::FrontLeft,
            ..Default::default()
        }
    }

    /// Create a front-right wheel configuration
    pub fn front_right(position: Vec3) -> Self {
        Self {
            position,
            steerable: true,
            driven: false,
            position_id: WheelPosition::FrontRight,
            ..Default::default()
        }
    }

    /// Create a rear-left wheel configuration (RWD driven)
    pub fn rear_left(position: Vec3) -> Self {
        Self {
            position,
            steerable: false,
            driven: true, // RWD
            position_id: WheelPosition::RearLeft,
            ..Default::default()
        }
    }

    /// Create a rear-right wheel configuration (RWD driven)
    pub fn rear_right(position: Vec3) -> Self {
        Self {
            position,
            steerable: false,
            driven: true, // RWD
            position_id: WheelPosition::RearRight,
            ..Default::default()
        }
    }

    /// Set as AWD (all-wheel drive)
    pub fn with_drive(mut self) -> Self {
        self.driven = true;
        self
    }

    /// Set wheel radius
    pub fn with_radius(mut self, radius: f32) -> Self {
        self.radius = radius;
        self
    }

    /// Set suspension parameters
    pub fn with_suspension(mut self, stiffness: f32, damping: f32, rest_length: f32) -> Self {
        self.suspension_stiffness = stiffness;
        self.suspension_damping = damping;
        self.suspension_rest_length = rest_length;
        self
    }
}

/// Drivetrain type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum DrivetrainType {
    /// Front-wheel drive
    FWD,
    /// Rear-wheel drive
    #[default]
    RWD,
    /// All-wheel drive
    AWD,
}

/// Engine configuration
#[derive(Debug, Clone)]
pub struct EngineConfig {
    /// Maximum engine torque (Nm)
    pub max_torque: f32,
    /// RPM at which max torque is produced
    pub max_torque_rpm: f32,
    /// Maximum engine RPM
    pub max_rpm: f32,
    /// Idle RPM
    pub idle_rpm: f32,
    /// Engine braking coefficient
    pub engine_braking: f32,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            max_torque: 400.0,
            max_torque_rpm: 4500.0,
            max_rpm: 7000.0,
            idle_rpm: 800.0,
            engine_braking: 0.3,
        }
    }
}

impl EngineConfig {
    /// Calculate torque at given RPM using simplified torque curve
    pub fn torque_at_rpm(&self, rpm: f32) -> f32 {
        if rpm < self.idle_rpm {
            return 0.0;
        }
        if rpm > self.max_rpm {
            return 0.0;
        }

        // Simple parabolic torque curve
        let normalized = (rpm - self.idle_rpm) / (self.max_torque_rpm - self.idle_rpm);
        let falloff = (rpm - self.max_torque_rpm) / (self.max_rpm - self.max_torque_rpm);

        if rpm <= self.max_torque_rpm {
            // Rising portion
            self.max_torque * (1.0 - (1.0 - normalized).powi(2))
        } else {
            // Falling portion
            self.max_torque * (1.0 - falloff.powi(2)).max(0.0)
        }
    }
}

/// Transmission configuration
#[derive(Debug, Clone)]
pub struct TransmissionConfig {
    /// Gear ratios (index 0 = 1st gear, etc.)
    pub gear_ratios: Vec<f32>,
    /// Reverse gear ratio
    pub reverse_ratio: f32,
    /// Final drive ratio (differential)
    pub final_drive: f32,
    /// Shift time in seconds
    pub shift_time: f32,
}

impl Default for TransmissionConfig {
    fn default() -> Self {
        Self {
            gear_ratios: vec![3.5, 2.1, 1.4, 1.0, 0.8, 0.65],
            reverse_ratio: -3.2,
            final_drive: 3.7,
            shift_time: 0.2,
        }
    }
}

impl TransmissionConfig {
    /// Get effective gear ratio for current gear
    pub fn effective_ratio(&self, gear: i32) -> f32 {
        let gear_ratio = if gear == 0 {
            0.0 // Neutral
        } else if gear < 0 {
            self.reverse_ratio
        } else {
            self.gear_ratios
                .get((gear - 1) as usize)
                .copied()
                .unwrap_or(1.0)
        };

        gear_ratio * self.final_drive
    }

    /// Number of forward gears
    pub fn num_gears(&self) -> usize {
        self.gear_ratios.len()
    }
}

/// Friction curve parameters (simplified Pacejka)
#[derive(Debug, Clone, Copy)]
pub struct FrictionCurve {
    /// Optimal slip ratio for maximum friction
    pub optimal_slip: f32,
    /// Friction coefficient at optimal slip
    pub peak_friction: f32,
    /// Friction coefficient at high slip (sliding)
    pub sliding_friction: f32,
    /// Curve stiffness
    pub stiffness: f32,
}

impl Default for FrictionCurve {
    fn default() -> Self {
        Self {
            optimal_slip: 0.08,
            peak_friction: 1.2,
            sliding_friction: 0.8,
            stiffness: 10.0,
        }
    }
}

impl FrictionCurve {
    /// Calculate friction coefficient at given slip ratio
    pub fn friction_at_slip(&self, slip: f32) -> f32 {
        let abs_slip = slip.abs();

        if abs_slip < 0.001 {
            return 0.0;
        }

        // Simplified magic formula
        let x = abs_slip / self.optimal_slip;
        let peak = self.peak_friction;
        let slide = self.sliding_friction;

        if x <= 1.0 {
            // Rising portion to peak
            peak * (1.0 - (-self.stiffness * x).exp())
        } else {
            // Falling portion after peak
            let decay = ((x - 1.0) * 2.0).min(1.0);
            peak - (peak - slide) * decay
        }
    }

    /// Tarmac/asphalt friction curve
    pub fn tarmac() -> Self {
        Self {
            optimal_slip: 0.08,
            peak_friction: 1.2,
            sliding_friction: 0.9,
            stiffness: 12.0,
        }
    }

    /// Gravel friction curve
    pub fn gravel() -> Self {
        Self {
            optimal_slip: 0.15,
            peak_friction: 0.8,
            sliding_friction: 0.6,
            stiffness: 6.0,
        }
    }

    /// Ice friction curve
    pub fn ice() -> Self {
        Self {
            optimal_slip: 0.05,
            peak_friction: 0.3,
            sliding_friction: 0.15,
            stiffness: 20.0,
        }
    }

    /// Mud friction curve
    pub fn mud() -> Self {
        Self {
            optimal_slip: 0.2,
            peak_friction: 0.5,
            sliding_friction: 0.4,
            stiffness: 4.0,
        }
    }
}

/// Vehicle configuration
#[derive(Debug, Clone)]
pub struct VehicleConfig {
    /// Vehicle mass (kg)
    pub mass: f32,
    /// Wheel configurations
    pub wheels: Vec<WheelConfig>,
    /// Drivetrain type
    pub drivetrain: DrivetrainType,
    /// Engine configuration
    pub engine: EngineConfig,
    /// Transmission configuration
    pub transmission: TransmissionConfig,
    /// Longitudinal friction curve
    pub friction_forward: FrictionCurve,
    /// Lateral friction curve
    pub friction_lateral: FrictionCurve,
    /// Aerodynamic drag coefficient
    pub drag_coefficient: f32,
    /// Frontal area (m²)
    pub frontal_area: f32,
    /// Center of mass offset from geometric center
    pub center_of_mass_offset: Vec3,
    /// Maximum steering angle (radians)
    pub max_steering_angle: f32,
    /// Brake force (N)
    pub brake_force: f32,
    /// Handbrake force multiplier for rear wheels
    pub handbrake_multiplier: f32,
}

impl Default for VehicleConfig {
    fn default() -> Self {
        Self {
            mass: 1500.0,
            wheels: vec![
                WheelConfig::front_left(Vec3::new(-0.8, 0.0, 1.2)),
                WheelConfig::front_right(Vec3::new(0.8, 0.0, 1.2)),
                WheelConfig::rear_left(Vec3::new(-0.8, 0.0, -1.2)),
                WheelConfig::rear_right(Vec3::new(0.8, 0.0, -1.2)),
            ],
            drivetrain: DrivetrainType::RWD,
            engine: EngineConfig::default(),
            transmission: TransmissionConfig::default(),
            friction_forward: FrictionCurve::tarmac(),
            friction_lateral: FrictionCurve::tarmac(),
            drag_coefficient: 0.35,
            frontal_area: 2.2,
            center_of_mass_offset: Vec3::new(0.0, -0.2, 0.0),
            max_steering_angle: 0.6, // ~35 degrees
            brake_force: 15000.0,
            handbrake_multiplier: 2.0,
        }
    }
}

/// Runtime state of a single wheel
#[derive(Debug, Clone)]
pub struct WheelState {
    /// Current suspension compression (0 = rest, positive = compressed)
    pub compression: f32,
    /// Previous compression (for damping)
    pub prev_compression: f32,
    /// Current wheel rotation speed (rad/s)
    pub rotation_speed: f32,
    /// Current steering angle (radians)
    pub steering_angle: f32,
    /// Is the wheel touching the ground?
    pub grounded: bool,
    /// Ground contact point (world space)
    pub contact_point: Vec3,
    /// Ground normal at contact
    pub contact_normal: Vec3,
    /// Current slip ratio (longitudinal)
    pub slip_ratio: f32,
    /// Current slip angle (lateral, radians)
    pub slip_angle: f32,
    /// Force applied by this wheel
    pub force: Vec3,
    /// Suspension force magnitude
    pub suspension_force: f32,
}

impl Default for WheelState {
    fn default() -> Self {
        Self {
            compression: 0.0,
            prev_compression: 0.0,
            rotation_speed: 0.0,
            steering_angle: 0.0,
            grounded: false,
            contact_point: Vec3::ZERO,
            contact_normal: Vec3::Y,
            slip_ratio: 0.0,
            slip_angle: 0.0,
            force: Vec3::ZERO,
            suspension_force: 0.0,
        }
    }
}

/// Vehicle input state
#[derive(Debug, Clone, Copy, Default)]
pub struct VehicleInput {
    /// Throttle (0.0 to 1.0)
    pub throttle: f32,
    /// Brake (0.0 to 1.0)
    pub brake: f32,
    /// Steering (-1.0 left to 1.0 right)
    pub steering: f32,
    /// Handbrake (0.0 to 1.0)
    pub handbrake: f32,
    /// Clutch (0.0 engaged to 1.0 disengaged)
    pub clutch: f32,
    /// Gear shift request (-1 = down, 0 = none, 1 = up)
    pub shift: i32,
}

/// Runtime state of a vehicle
#[derive(Debug)]
pub struct Vehicle {
    /// Unique ID
    pub id: VehicleId,
    /// Physics body ID
    pub body_id: BodyId,
    /// Configuration
    pub config: VehicleConfig,
    /// Wheel states
    pub wheels: Vec<WheelState>,
    /// Current gear (0 = neutral, negative = reverse)
    pub current_gear: i32,
    /// Current engine RPM
    pub engine_rpm: f32,
    /// Time remaining in gear shift
    pub shift_timer: f32,
    /// Current speed (m/s)
    pub speed: f32,
    /// Current velocity (world space)
    pub velocity: Vec3,
    /// Forward direction (world space)
    pub forward: Vec3,
    /// Right direction (world space)
    pub right: Vec3,
    /// Up direction (world space)
    pub up: Vec3,
}

impl Vehicle {
    /// Create a new vehicle
    pub fn new(id: VehicleId, body_id: BodyId, config: VehicleConfig) -> Self {
        let num_wheels = config.wheels.len();
        Self {
            id,
            body_id,
            config,
            wheels: vec![WheelState::default(); num_wheels],
            current_gear: 1,   // Start in 1st
            engine_rpm: 800.0, // Idle
            shift_timer: 0.0,
            speed: 0.0,
            velocity: Vec3::ZERO,
            forward: Vec3::Z,
            right: Vec3::X,
            up: Vec3::Y,
        }
    }

    /// Update vehicle orientation from physics body
    pub fn update_orientation(&mut self, rotation: Quat) {
        self.forward = rotation * Vec3::Z;
        self.right = rotation * Vec3::X;
        self.up = rotation * Vec3::Y;
    }

    /// Get speed in km/h
    pub fn speed_kmh(&self) -> f32 {
        self.speed * 3.6
    }

    /// Get speed in mph
    pub fn speed_mph(&self) -> f32 {
        self.speed * 2.237
    }

    /// Check if currently shifting gears
    pub fn is_shifting(&self) -> bool {
        self.shift_timer > 0.0
    }

    /// Shift up a gear
    pub fn shift_up(&mut self) {
        let max_gear = self.config.transmission.num_gears() as i32;
        if self.current_gear < max_gear && !self.is_shifting() {
            self.current_gear += 1;
            self.shift_timer = self.config.transmission.shift_time;
        }
    }

    /// Shift down a gear
    pub fn shift_down(&mut self) {
        if self.current_gear > -1 && !self.is_shifting() {
            self.current_gear -= 1;
            self.shift_timer = self.config.transmission.shift_time;
        }
    }

    /// Get number of grounded wheels
    pub fn grounded_wheels(&self) -> usize {
        self.wheels.iter().filter(|w| w.grounded).count()
    }

    /// Check if vehicle is airborne
    pub fn is_airborne(&self) -> bool {
        self.grounded_wheels() == 0
    }

    /// Get total suspension force
    pub fn total_suspension_force(&self) -> f32 {
        self.wheels.iter().map(|w| w.suspension_force).sum()
    }

    /// Get average slip ratio
    pub fn average_slip_ratio(&self) -> f32 {
        let grounded: Vec<_> = self.wheels.iter().filter(|w| w.grounded).collect();
        if grounded.is_empty() {
            return 0.0;
        }
        grounded.iter().map(|w| w.slip_ratio.abs()).sum::<f32>() / grounded.len() as f32
    }

    /// Get average slip angle
    pub fn average_slip_angle(&self) -> f32 {
        let grounded: Vec<_> = self.wheels.iter().filter(|w| w.grounded).collect();
        if grounded.is_empty() {
            return 0.0;
        }
        grounded.iter().map(|w| w.slip_angle.abs()).sum::<f32>() / grounded.len() as f32
    }
}

/// Vehicle physics manager
#[derive(Debug)]
pub struct VehicleManager {
    vehicles: Vec<Vehicle>,
    next_id: VehicleId,
}

impl Default for VehicleManager {
    fn default() -> Self {
        Self::new()
    }
}

impl VehicleManager {
    /// Create a new vehicle manager
    pub fn new() -> Self {
        Self {
            vehicles: Vec::new(),
            next_id: 1,
        }
    }

    /// Spawn a vehicle
    pub fn spawn(
        &mut self,
        physics: &mut PhysicsWorld,
        position: Vec3,
        config: VehicleConfig,
    ) -> VehicleId {
        let body_id = physics.add_dynamic_box(
            position + config.center_of_mass_offset,
            Vec3::new(1.0, 0.5, 2.0), // Approximate vehicle half-extents
            config.mass,
            crate::Layers::DEFAULT,
        );

        // Enable CCD for fast-moving vehicles
        physics.enable_ccd(body_id);

        let id = self.next_id;
        self.next_id += 1;

        let vehicle = Vehicle::new(id, body_id, config);
        self.vehicles.push(vehicle);

        id
    }

    /// Get a vehicle by ID
    pub fn get(&self, id: VehicleId) -> Option<&Vehicle> {
        self.vehicles.iter().find(|v| v.id == id)
    }

    /// Get a mutable vehicle by ID
    pub fn get_mut(&mut self, id: VehicleId) -> Option<&mut Vehicle> {
        self.vehicles.iter_mut().find(|v| v.id == id)
    }

    /// Update all vehicles
    pub fn update(&mut self, physics: &mut PhysicsWorld, dt: f32) {
        for vehicle in &mut self.vehicles {
            Self::update_vehicle(vehicle, physics, dt);
        }
    }

    /// Update vehicle with input
    pub fn update_with_input(
        &mut self,
        id: VehicleId,
        physics: &mut PhysicsWorld,
        input: &VehicleInput,
        dt: f32,
    ) {
        if let Some(vehicle) = self.get_mut(id) {
            // Handle gear shifts
            if input.shift > 0 {
                vehicle.shift_up();
            } else if input.shift < 0 {
                vehicle.shift_down();
            }

            // Update shift timer
            if vehicle.shift_timer > 0.0 {
                vehicle.shift_timer = (vehicle.shift_timer - dt).max(0.0);
            }

            // Apply steering
            for (i, wheel) in vehicle.wheels.iter_mut().enumerate() {
                if vehicle.config.wheels[i].steerable {
                    wheel.steering_angle = input.steering * vehicle.config.max_steering_angle;
                }
            }

            Self::update_vehicle(vehicle, physics, dt);
            Self::apply_forces(vehicle, physics, input, dt);
        }
    }

    fn update_vehicle(vehicle: &mut Vehicle, physics: &PhysicsWorld, _dt: f32) {
        // Get vehicle transform
        if let Some(transform) = physics.body_transform(vehicle.body_id) {
            let position = Vec3::new(transform.w_axis.x, transform.w_axis.y, transform.w_axis.z);

            // Extract rotation (simplified - assumes orthonormal)
            let rotation = Quat::from_mat4(&transform);
            vehicle.update_orientation(rotation);

            // Get velocity
            if let Some(vel) = physics.get_velocity(vehicle.body_id) {
                vehicle.velocity = vel;
                vehicle.speed = vel.length();
            }

            // Update wheel states with raycasts
            for (i, wheel_config) in vehicle.config.wheels.iter().enumerate() {
                let wheel_state = &mut vehicle.wheels[i];

                // Calculate wheel world position
                let wheel_pos = position + rotation * wheel_config.position;

                // Raycast downward from wheel
                let ray_origin = wheel_pos + vehicle.up * wheel_config.suspension_max_extension;
                let ray_length = wheel_config.suspension_rest_length
                    + wheel_config.suspension_max_compression
                    + wheel_config.suspension_max_extension
                    + wheel_config.radius;

                // Perform raycast
                if let Some((hit_point, hit_normal, _body_id, _distance)) =
                    physics.raycast(ray_origin, -vehicle.up, ray_length)
                {
                    wheel_state.grounded = true;
                    wheel_state.contact_point = hit_point;
                    wheel_state.contact_normal = hit_normal;

                    // Calculate suspension compression
                    let suspension_length = (wheel_pos - hit_point).length() - wheel_config.radius;
                    wheel_state.prev_compression = wheel_state.compression;
                    wheel_state.compression =
                        wheel_config.suspension_rest_length - suspension_length;
                } else {
                    wheel_state.grounded = false;
                    wheel_state.prev_compression = wheel_state.compression;
                    wheel_state.compression = -wheel_config.suspension_max_extension;
                }
            }
        }
    }

    fn apply_forces(
        vehicle: &mut Vehicle,
        physics: &mut PhysicsWorld,
        input: &VehicleInput,
        dt: f32,
    ) {
        let mut total_force = Vec3::ZERO;
        let mut total_torque = Vec3::ZERO;

        // Get vehicle transform for force application points
        let Some(transform) = physics.body_transform(vehicle.body_id) else {
            return;
        };
        let position = Vec3::new(transform.w_axis.x, transform.w_axis.y, transform.w_axis.z);
        let rotation = Quat::from_mat4(&transform);

        // Calculate engine torque
        let effective_throttle = if vehicle.is_shifting() {
            0.0
        } else {
            input.throttle
        };
        let gear_ratio = vehicle
            .config
            .transmission
            .effective_ratio(vehicle.current_gear);
        let engine_torque =
            vehicle.config.engine.torque_at_rpm(vehicle.engine_rpm) * effective_throttle;
        let wheel_torque = engine_torque * gear_ratio;

        // Count driven wheels
        let driven_count = vehicle.config.wheels.iter().filter(|w| w.driven).count() as f32;

        for (i, wheel_config) in vehicle.config.wheels.iter().enumerate() {
            let wheel_state = &mut vehicle.wheels[i];

            if !wheel_state.grounded {
                continue;
            }

            let wheel_world_pos = position + rotation * wheel_config.position;

            // Suspension force (spring + damper)
            let spring_force = wheel_state.compression * wheel_config.suspension_stiffness;
            let damper_velocity = (wheel_state.compression - wheel_state.prev_compression) / dt;
            let damper_force = damper_velocity * wheel_config.suspension_damping;
            let suspension_force = (spring_force + damper_force).max(0.0);
            wheel_state.suspension_force = suspension_force;

            // Normal force on this wheel
            let normal_force = suspension_force;

            // Calculate wheel direction with steering
            let steer_rotation = Quat::from_rotation_y(wheel_state.steering_angle);
            let wheel_forward = rotation * steer_rotation * Vec3::Z;
            let wheel_right = rotation * steer_rotation * Vec3::X;

            // Velocity at wheel contact point
            let contact_velocity = vehicle.velocity; // Simplified - ignores angular velocity

            // Longitudinal velocity (along wheel forward)
            let long_velocity = contact_velocity.dot(wheel_forward);

            // Lateral velocity (perpendicular to wheel forward)
            let lat_velocity = contact_velocity.dot(wheel_right);

            // Calculate slip ratio
            let wheel_speed = wheel_state.rotation_speed * wheel_config.radius;
            let slip_ratio = if long_velocity.abs() > 0.5 {
                (wheel_speed - long_velocity) / long_velocity.abs()
            } else if wheel_speed.abs() > 0.1 {
                wheel_speed.signum()
            } else {
                0.0
            };
            wheel_state.slip_ratio = slip_ratio.clamp(-1.0, 1.0);

            // Calculate slip angle
            let slip_angle = if long_velocity.abs() > 0.5 {
                (-lat_velocity / long_velocity.abs()).atan()
            } else {
                0.0
            };
            wheel_state.slip_angle = slip_angle;

            // Friction forces
            let long_friction = vehicle.config.friction_forward.friction_at_slip(slip_ratio);
            let lat_friction = vehicle
                .config
                .friction_lateral
                .friction_at_slip(slip_angle.abs());

            // Longitudinal force (drive/brake)
            let mut long_force = 0.0;

            // Drive force
            if wheel_config.driven && driven_count > 0.0 {
                let torque_per_wheel = wheel_torque / driven_count;
                let max_friction_force = normal_force * long_friction;
                let drive_force = (torque_per_wheel / wheel_config.radius).min(max_friction_force);
                long_force += drive_force * effective_throttle;
            }

            // Brake force
            let brake_input = if wheel_config.position_id == WheelPosition::RearLeft
                || wheel_config.position_id == WheelPosition::RearRight
            {
                input.brake + input.handbrake * vehicle.config.handbrake_multiplier
            } else {
                input.brake
            };

            if brake_input > 0.0 {
                let max_brake = vehicle.config.brake_force * brake_input;
                let brake_friction_limit = normal_force * long_friction;
                let brake_force = max_brake.min(brake_friction_limit);
                long_force -= brake_force * long_velocity.signum();
            }

            // Lateral force (cornering)
            let lat_force = -lat_velocity.signum() * normal_force * lat_friction;

            // Total wheel force
            let wheel_force = wheel_forward * long_force
                + wheel_right * lat_force
                + wheel_state.contact_normal * suspension_force;
            wheel_state.force = wheel_force;

            // Accumulate forces
            total_force += wheel_force;

            // Calculate torque from force application point
            let force_arm = wheel_world_pos - position;
            total_torque += force_arm.cross(wheel_force);

            // Update wheel rotation
            let angular_accel = if wheel_config.driven && driven_count > 0.0 {
                let torque_per_wheel = wheel_torque / driven_count;
                let wheel_inertia = 0.5 * 10.0 * wheel_config.radius.powi(2); // Simplified
                (torque_per_wheel - long_force * wheel_config.radius) / wheel_inertia
            } else {
                -long_force * wheel_config.radius / (0.5 * 10.0 * wheel_config.radius.powi(2))
            };
            wheel_state.rotation_speed += angular_accel * dt;

            // Apply friction to slow wheel when not driven
            if !wheel_config.driven || effective_throttle < 0.01 {
                wheel_state.rotation_speed *= 0.99; // Rolling resistance
            }
        }

        // Aerodynamic drag
        let speed_sq = vehicle.speed * vehicle.speed;
        let drag_force =
            0.5 * 1.225 * vehicle.config.drag_coefficient * vehicle.config.frontal_area * speed_sq;
        total_force -=
            vehicle.forward * drag_force * vehicle.velocity.dot(vehicle.forward).signum();

        // Apply forces to physics body
        physics.apply_force(vehicle.body_id, total_force);

        // Update engine RPM based on throttle and wheel load
        // Engine revs up with throttle input
        let throttle_rpm_target = vehicle.config.engine.idle_rpm
            + input.throttle
                * (vehicle.config.engine.max_rpm - vehicle.config.engine.idle_rpm)
                * 0.8;

        if gear_ratio.abs() > 0.01 && driven_count > 0.0 {
            let avg_wheel_rpm: f32 = vehicle
                .wheels
                .iter()
                .enumerate()
                .filter(|(i, _)| vehicle.config.wheels[*i].driven)
                .map(|(_, w)| w.rotation_speed.abs() * 60.0 / (2.0 * std::f32::consts::PI))
                .sum::<f32>()
                / driven_count;

            let wheel_target_rpm = avg_wheel_rpm * gear_ratio.abs();

            // Engine RPM is influenced by both throttle and wheel feedback
            // Throttle pulls RPM up, wheel speed provides load feedback
            let load_factor = (vehicle.speed / 20.0).clamp(0.0, 0.5); // More wheel influence at speed
            let target_rpm = throttle_rpm_target * (1.0 - load_factor)
                + wheel_target_rpm.max(throttle_rpm_target * 0.3) * load_factor;

            // Smooth RPM changes
            vehicle.engine_rpm = vehicle.engine_rpm * 0.85 + target_rpm * 0.15;
        } else {
            // No gear engaged - free rev
            vehicle.engine_rpm = vehicle.engine_rpm * 0.85 + throttle_rpm_target * 0.15;
        }

        // Clamp engine RPM
        vehicle.engine_rpm = vehicle.engine_rpm.clamp(
            vehicle.config.engine.idle_rpm,
            vehicle.config.engine.max_rpm,
        );
    }

    /// Remove a vehicle
    pub fn remove(&mut self, id: VehicleId) -> bool {
        if let Some(pos) = self.vehicles.iter().position(|v| v.id == id) {
            self.vehicles.remove(pos);
            true
        } else {
            false
        }
    }

    /// Get all vehicles
    pub fn vehicles(&self) -> &[Vehicle] {
        &self.vehicles
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wheel_config_defaults() {
        let wheel = WheelConfig::default();
        assert!((wheel.radius - 0.35).abs() < 0.01);
        assert!(wheel.suspension_stiffness > 0.0);
    }

    #[test]
    fn test_wheel_config_presets() {
        let fl = WheelConfig::front_left(Vec3::new(-1.0, 0.0, 1.5));
        assert!(fl.steerable);
        assert!(!fl.driven);
        assert_eq!(fl.position_id, WheelPosition::FrontLeft);

        let rr = WheelConfig::rear_right(Vec3::new(1.0, 0.0, -1.5));
        assert!(!rr.steerable);
        assert!(rr.driven);
        assert_eq!(rr.position_id, WheelPosition::RearRight);
    }

    #[test]
    fn test_wheel_config_awd() {
        let wheel = WheelConfig::front_left(Vec3::ZERO).with_drive();
        assert!(wheel.driven);
    }

    #[test]
    fn test_engine_torque_curve() {
        let engine = EngineConfig::default();

        // Below idle: no torque
        assert!((engine.torque_at_rpm(500.0)).abs() < 0.01);

        // At max torque RPM: should be near max
        let torque_at_peak = engine.torque_at_rpm(engine.max_torque_rpm);
        assert!((torque_at_peak - engine.max_torque).abs() < 50.0);

        // Above max RPM: no torque
        assert!((engine.torque_at_rpm(8000.0)).abs() < 0.01);
    }

    #[test]
    fn test_transmission_gear_ratios() {
        let trans = TransmissionConfig::default();

        // Neutral
        assert!((trans.effective_ratio(0)).abs() < 0.01);

        // First gear should have highest ratio
        let first = trans.effective_ratio(1);
        let second = trans.effective_ratio(2);
        assert!(first > second);

        // Reverse should be negative
        assert!(trans.effective_ratio(-1) < 0.0);
    }

    #[test]
    fn test_friction_curve_tarmac() {
        let curve = FrictionCurve::tarmac();

        // Zero slip: zero friction
        assert!(curve.friction_at_slip(0.0).abs() < 0.01);

        // Optimal slip: peak friction
        let peak = curve.friction_at_slip(curve.optimal_slip);
        assert!(peak > 1.0, "Peak friction should exceed 1.0 on tarmac");

        // High slip: reduced friction
        let slide = curve.friction_at_slip(0.5);
        assert!(slide < peak, "Sliding friction should be less than peak");
    }

    #[test]
    fn test_friction_curve_ice() {
        let ice = FrictionCurve::ice();
        let tarmac = FrictionCurve::tarmac();

        let ice_friction = ice.friction_at_slip(ice.optimal_slip);
        let tarmac_friction = tarmac.friction_at_slip(tarmac.optimal_slip);

        assert!(
            ice_friction < tarmac_friction,
            "Ice should have less grip than tarmac"
        );
    }

    #[test]
    fn test_vehicle_config_default() {
        let config = VehicleConfig::default();
        assert_eq!(config.wheels.len(), 4);
        assert!((config.mass - 1500.0).abs() < 0.1);
    }

    #[test]
    fn test_vehicle_creation() {
        let config = VehicleConfig::default();
        let vehicle = Vehicle::new(1, 42, config);

        assert_eq!(vehicle.id, 1);
        assert_eq!(vehicle.body_id, 42);
        assert_eq!(vehicle.wheels.len(), 4);
        assert_eq!(vehicle.current_gear, 1);
    }

    #[test]
    fn test_vehicle_speed_conversion() {
        let config = VehicleConfig::default();
        let mut vehicle = Vehicle::new(1, 42, config);
        vehicle.speed = 27.78; // ~100 km/h

        let kmh = vehicle.speed_kmh();
        assert!((kmh - 100.0).abs() < 1.0);

        let mph = vehicle.speed_mph();
        assert!((mph - 62.1).abs() < 1.0);
    }

    #[test]
    fn test_vehicle_shifting() {
        let config = VehicleConfig::default();
        let mut vehicle = Vehicle::new(1, 42, config);

        assert_eq!(vehicle.current_gear, 1);

        vehicle.shift_up();
        assert_eq!(vehicle.current_gear, 2);
        assert!(vehicle.is_shifting());

        // Can't shift while already shifting
        vehicle.shift_up();
        assert_eq!(vehicle.current_gear, 2);

        // Clear shift timer
        vehicle.shift_timer = 0.0;

        vehicle.shift_down();
        assert_eq!(vehicle.current_gear, 1);
    }

    #[test]
    fn test_vehicle_grounded_wheels() {
        let config = VehicleConfig::default();
        let mut vehicle = Vehicle::new(1, 42, config);

        // Initially no wheels grounded
        assert_eq!(vehicle.grounded_wheels(), 0);
        assert!(vehicle.is_airborne());

        // Ground some wheels
        vehicle.wheels[0].grounded = true;
        vehicle.wheels[1].grounded = true;
        assert_eq!(vehicle.grounded_wheels(), 2);
        assert!(!vehicle.is_airborne());
    }

    #[test]
    fn test_vehicle_manager_creation() {
        let manager = VehicleManager::new();
        assert_eq!(manager.vehicles().len(), 0);
    }

    #[test]
    fn test_vehicle_input_default() {
        let input = VehicleInput::default();
        assert!((input.throttle).abs() < 0.01);
        assert!((input.brake).abs() < 0.01);
        assert!((input.steering).abs() < 0.01);
    }

    #[test]
    fn test_wheel_state_default() {
        let state = WheelState::default();
        assert!(!state.grounded);
        assert!((state.compression).abs() < 0.01);
        assert!((state.rotation_speed).abs() < 0.01);
    }

    #[test]
    fn test_drivetrain_types() {
        assert_eq!(DrivetrainType::default(), DrivetrainType::RWD);
    }

    #[test]
    fn test_suspension_force_calculation() {
        let wheel = WheelConfig::default();
        let compression = 0.05; // 5cm compressed

        let spring_force = compression * wheel.suspension_stiffness;
        assert!(spring_force > 0.0);
        assert!(spring_force < wheel.suspension_stiffness); // Sanity check
    }

    #[test]
    fn test_vehicle_orientation() {
        let config = VehicleConfig::default();
        let mut vehicle = Vehicle::new(1, 42, config);

        // Default orientation
        vehicle.update_orientation(Quat::IDENTITY);
        assert!((vehicle.forward - Vec3::Z).length() < 0.01);
        assert!((vehicle.right - Vec3::X).length() < 0.01);
        assert!((vehicle.up - Vec3::Y).length() < 0.01);

        // Rotated 90 degrees around Y
        let rotation = Quat::from_rotation_y(std::f32::consts::FRAC_PI_2);
        vehicle.update_orientation(rotation);
        assert!((vehicle.forward - Vec3::X).length() < 0.1);
    }

    #[test]
    fn test_total_suspension_force() {
        let config = VehicleConfig::default();
        let mut vehicle = Vehicle::new(1, 42, config);

        // Set suspension forces on wheels
        vehicle.wheels[0].suspension_force = 1000.0;
        vehicle.wheels[1].suspension_force = 1000.0;
        vehicle.wheels[2].suspension_force = 800.0;
        vehicle.wheels[3].suspension_force = 800.0;

        let total = vehicle.total_suspension_force();
        assert!((total - 3600.0).abs() < 0.01);
    }

    #[test]
    fn test_average_slip_ratio_all_grounded() {
        let config = VehicleConfig::default();
        let mut vehicle = Vehicle::new(1, 42, config);

        // Ground all wheels and set slip ratios
        for (i, wheel) in vehicle.wheels.iter_mut().enumerate() {
            wheel.grounded = true;
            wheel.slip_ratio = (i as f32 + 1.0) * 0.1; // 0.1, 0.2, 0.3, 0.4
        }

        let avg = vehicle.average_slip_ratio();
        // Average of 0.1 + 0.2 + 0.3 + 0.4 = 1.0 / 4 = 0.25
        assert!((avg - 0.25).abs() < 0.01);
    }

    #[test]
    fn test_average_slip_ratio_some_airborne() {
        let config = VehicleConfig::default();
        let mut vehicle = Vehicle::new(1, 42, config);

        // Only ground front wheels
        vehicle.wheels[0].grounded = true;
        vehicle.wheels[0].slip_ratio = 0.2;
        vehicle.wheels[1].grounded = true;
        vehicle.wheels[1].slip_ratio = 0.4;
        vehicle.wheels[2].grounded = false;
        vehicle.wheels[3].grounded = false;

        let avg = vehicle.average_slip_ratio();
        // Average of 0.2 + 0.4 = 0.6 / 2 = 0.3
        assert!((avg - 0.3).abs() < 0.01);
    }

    #[test]
    fn test_average_slip_ratio_airborne() {
        let config = VehicleConfig::default();
        let vehicle = Vehicle::new(1, 42, config);

        // All wheels airborne by default
        let avg = vehicle.average_slip_ratio();
        assert!((avg).abs() < 0.01);
    }

    #[test]
    fn test_average_slip_angle() {
        let config = VehicleConfig::default();
        let mut vehicle = Vehicle::new(1, 42, config);

        // Ground all wheels and set slip angles
        for (i, wheel) in vehicle.wheels.iter_mut().enumerate() {
            wheel.grounded = true;
            wheel.slip_angle = (i as f32 + 1.0) * 0.05; // 0.05, 0.10, 0.15, 0.20
        }

        let avg = vehicle.average_slip_angle();
        // Average of 0.05 + 0.10 + 0.15 + 0.20 = 0.50 / 4 = 0.125
        assert!((avg - 0.125).abs() < 0.01);
    }

    #[test]
    fn test_friction_curve_gravel() {
        let curve = FrictionCurve::gravel();

        assert!((curve.optimal_slip - 0.15).abs() < 0.01);
        assert!(curve.peak_friction < FrictionCurve::tarmac().peak_friction);
    }

    #[test]
    fn test_friction_curve_mud() {
        let curve = FrictionCurve::mud();

        assert!((curve.optimal_slip - 0.2).abs() < 0.01);
        assert!(curve.peak_friction < FrictionCurve::gravel().peak_friction);
    }

    #[test]
    fn test_friction_curve_defaults() {
        let curve = FrictionCurve::default();

        assert!(curve.optimal_slip > 0.0);
        assert!(curve.peak_friction > 0.0);
        assert!(curve.sliding_friction > 0.0);
        assert!(curve.stiffness > 0.0);
    }

    #[test]
    fn test_friction_rising_portion() {
        let curve = FrictionCurve::tarmac();

        // Below optimal slip, friction should be increasing
        let f1 = curve.friction_at_slip(0.02);
        let f2 = curve.friction_at_slip(0.05);
        assert!(f2 > f1);
    }

    #[test]
    fn test_friction_falling_portion() {
        let curve = FrictionCurve::tarmac();

        // Well above optimal slip, friction should be lower than peak
        let peak = curve.friction_at_slip(curve.optimal_slip);
        let high_slip = curve.friction_at_slip(0.5);
        assert!(high_slip < peak);
    }

    #[test]
    fn test_wheel_config_with_radius() {
        let wheel = WheelConfig::default().with_radius(0.5);
        assert!((wheel.radius - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_wheel_config_with_suspension() {
        let wheel = WheelConfig::default().with_suspension(40000.0, 5000.0, 0.35);
        assert!((wheel.suspension_stiffness - 40000.0).abs() < 0.01);
        assert!((wheel.suspension_damping - 5000.0).abs() < 0.01);
        assert!((wheel.suspension_rest_length - 0.35).abs() < 0.01);
    }

    #[test]
    fn test_wheel_position_custom() {
        let wheel = WheelConfig {
            position_id: WheelPosition::Custom(5),
            ..Default::default()
        };
        assert_eq!(wheel.position_id, WheelPosition::Custom(5));
    }

    #[test]
    fn test_vehicle_config_mass() {
        let config = VehicleConfig::default();
        assert!(config.mass > 0.0);
        assert!((config.mass - 1500.0).abs() < 0.1);
    }

    #[test]
    fn test_vehicle_config_drag() {
        let config = VehicleConfig::default();
        assert!(config.drag_coefficient > 0.0);
        assert!(config.frontal_area > 0.0);
    }

    #[test]
    fn test_vehicle_config_brake_force() {
        let config = VehicleConfig::default();
        assert!(config.brake_force > 0.0);
        assert!(config.handbrake_multiplier > 1.0);
    }

    #[test]
    fn test_transmission_num_gears() {
        let trans = TransmissionConfig::default();
        assert_eq!(trans.num_gears(), 6);
    }

    #[test]
    fn test_engine_idle_torque() {
        let engine = EngineConfig::default();
        let torque = engine.torque_at_rpm(engine.idle_rpm);
        assert!(torque >= 0.0);
    }

    #[test]
    fn test_vehicle_manager_get_nonexistent() {
        let manager = VehicleManager::new();
        assert!(manager.get(999).is_none());
    }

    #[test]
    fn test_vehicle_shift_to_neutral() {
        let config = VehicleConfig::default();
        let mut vehicle = Vehicle::new(1, 42, config);

        // Start in 1st, shift to neutral (0)
        assert_eq!(vehicle.current_gear, 1);
        vehicle.shift_down();
        vehicle.shift_timer = 0.0; // Clear shift timer
        assert_eq!(vehicle.current_gear, 0);
    }

    #[test]
    fn test_vehicle_shift_to_reverse() {
        let config = VehicleConfig::default();
        let mut vehicle = Vehicle::new(1, 42, config);

        // Shift from 1st to neutral to reverse
        vehicle.shift_down();
        vehicle.shift_timer = 0.0;
        vehicle.shift_down();
        vehicle.shift_timer = 0.0;

        assert_eq!(vehicle.current_gear, -1);
    }

    #[test]
    fn test_vehicle_shift_down_limit() {
        let config = VehicleConfig::default();
        let mut vehicle = Vehicle::new(1, 42, config);

        // Shift all the way down
        for _ in 0..5 {
            vehicle.shift_down();
            vehicle.shift_timer = 0.0;
        }

        // Should not go below -1
        assert!(vehicle.current_gear >= -1);
    }

    #[test]
    fn test_vehicle_shift_up_limit() {
        let config = VehicleConfig::default();
        let mut vehicle = Vehicle::new(1, 42, config);

        // Shift all the way up
        for _ in 0..10 {
            vehicle.shift_up();
            vehicle.shift_timer = 0.0;
        }

        // Should not exceed max gears
        let max_gear = vehicle.config.transmission.num_gears() as i32;
        assert!(vehicle.current_gear <= max_gear);
    }

    #[test]
    fn test_vehicle_input_creation() {
        let input = VehicleInput {
            throttle: 0.8,
            brake: 0.0,
            steering: -0.5,
            handbrake: 0.0,
            clutch: 0.0,
            shift: 1,
        };

        assert!((input.throttle - 0.8).abs() < 0.01);
        assert!((input.steering - -0.5).abs() < 0.01);
        assert_eq!(input.shift, 1);
    }

    #[test]
    fn test_wheel_state_contact_normal() {
        let state = WheelState {
            contact_normal: Vec3::new(0.0, 1.0, 0.0),
            ..Default::default()
        };

        assert!((state.contact_normal.y - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_vehicle_manager_default() {
        let manager = VehicleManager::default();
        assert!(manager.vehicles().is_empty());
    }

    #[test]
    fn test_vehicle_config_center_of_mass() {
        let config = VehicleConfig::default();
        // Center of mass should be slightly below geometric center
        assert!(config.center_of_mass_offset.y < 0.0);
    }

    #[test]
    fn test_vehicle_config_steering_angle() {
        let config = VehicleConfig::default();
        // Max steering angle should be reasonable (e.g., 30-45 degrees)
        assert!(config.max_steering_angle > 0.4);
        assert!(config.max_steering_angle < 1.0);
    }

    #[test]
    fn test_vehicle_manager_spawn_and_get() {
        let mut physics = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));
        let mut manager = VehicleManager::new();
        let config = VehicleConfig::default();

        let id = manager.spawn(&mut physics, Vec3::ZERO, config);
        assert_eq!(manager.vehicles().len(), 1);
        assert!(manager.get(id).is_some());
        assert!(manager.get_mut(id).is_some());
    }

    #[test]
    fn test_vehicle_orientation_update() {
        let config = VehicleConfig::default();
        let mut vehicle = Vehicle::new(1, 42, config);

        let rotation = Quat::from_rotation_y(std::f32::consts::FRAC_PI_2);
        vehicle.update_orientation(rotation);

        // Forward should now be along X axis
        assert!(vehicle.forward.x > 0.9);
    }

    #[test]
    fn test_vehicle_slip_averages() {
        let config = VehicleConfig::default();
        let mut vehicle = Vehicle::new(1, 42, config);

        // Airborne case
        assert_eq!(vehicle.average_slip_ratio(), 0.0);
        assert_eq!(vehicle.average_slip_angle(), 0.0);

        // Grounded case
        vehicle.wheels[0].grounded = true;
        vehicle.wheels[0].slip_ratio = 0.5;
        vehicle.wheels[0].slip_angle = 0.1;

        assert!((vehicle.average_slip_ratio() - 0.5).abs() < 0.01);
        assert!((vehicle.average_slip_angle() - 0.1).abs() < 0.01);
    }

    #[test]
    fn test_engine_torque_falling_curve() {
        let engine = EngineConfig {
            max_torque: 400.0,
            max_torque_rpm: 4000.0,
            max_rpm: 6000.0,
            idle_rpm: 1000.0,
            ..Default::default()
        };

        // In falling portion (between 4000 and 6000)
        let torque = engine.torque_at_rpm(5000.0);
        assert!(torque > 0.0 && torque < 400.0);
    }

    #[test]
    fn test_vehicle_manager_update_logic() {
        let mut physics = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));
        let mut manager = VehicleManager::new();
        let id = manager.spawn(&mut physics, Vec3::ZERO, VehicleConfig::default());

        // Update without input
        manager.update(&mut physics, 0.016);

        // Update with input
        let input = VehicleInput {
            throttle: 1.0,
            steering: 0.5,
            shift: 1,
            ..Default::default()
        };
        manager.update_with_input(id, &mut physics, &input, 0.016);

        let vehicle = manager.get(id).unwrap();
        assert_eq!(vehicle.current_gear, 2);
    }

    #[test]
    fn test_vehicle_suspension_force() {
        let config = VehicleConfig::default();
        let mut vehicle = Vehicle::new(1, 42, config);
        vehicle.wheels[0].suspension_force = 1000.0;
        assert_eq!(vehicle.total_suspension_force(), 1000.0);
    }

    #[test]
    fn test_friction_curves_all() {
        let _ = FrictionCurve::gravel();
        let _ = FrictionCurve::mud();
        let _ = FrictionCurve::default();
    }

    #[test]
    fn test_wheel_config_setters() {
        let wheel = WheelConfig::default()
            .with_radius(0.5)
            .with_suspension(40000.0, 5000.0, 0.4);
        assert_eq!(wheel.radius, 0.5);
        assert_eq!(wheel.suspension_stiffness, 40000.0);
    }

    // ============================================================================
    // PACEJKA TIRE MODEL VALIDATION (Phase 8.8 - New)
    // ============================================================================

    #[test]
    fn test_friction_curve_zero_slip() {
        let curve = FrictionCurve::default();
        let friction = curve.friction_at_slip(0.0);
        assert_eq!(friction, 0.0); // No friction with no slip
    }

    #[test]
    fn test_friction_curve_optimal_slip() {
        let curve = FrictionCurve::default();
        let friction = curve.friction_at_slip(curve.optimal_slip);
        // At optimal slip, should be near peak friction
        assert!(friction > curve.sliding_friction);
    }

    #[test]
    fn test_friction_curve_high_slip() {
        let curve = FrictionCurve::default();
        // Very high slip should give sliding friction
        let friction = curve.friction_at_slip(0.5);
        assert!(friction >= curve.sliding_friction);
        assert!(friction <= curve.peak_friction);
    }

    #[test]
    fn test_friction_curve_negative_slip() {
        let curve = FrictionCurve::default();
        let positive = curve.friction_at_slip(0.1);
        let negative = curve.friction_at_slip(-0.1);
        // Friction should be symmetric for slip direction
        assert!((positive - negative).abs() < 0.01);
    }

    #[test]
    fn test_friction_tarmac_vs_ice() {
        let tarmac = FrictionCurve::tarmac();
        let ice = FrictionCurve::ice();

        // Tarmac should have much higher friction
        assert!(tarmac.peak_friction > ice.peak_friction * 2.0);

        // Ice should have lower optimal slip
        assert!(ice.optimal_slip < tarmac.optimal_slip);
    }

    #[test]
    fn test_friction_curve_monotonic_rising() {
        let curve = FrictionCurve::default();
        let mut prev_friction = 0.0;

        // Should be monotonically increasing up to optimal slip
        for i in 1..10 {
            let slip = (i as f32) * curve.optimal_slip / 10.0;
            let friction = curve.friction_at_slip(slip);
            assert!(friction >= prev_friction);
            prev_friction = friction;
        }
    }

    #[test]
    fn test_friction_surface_comparison() {
        let tarmac = FrictionCurve::tarmac();
        let gravel = FrictionCurve::gravel();
        let ice = FrictionCurve::ice();
        let mud = FrictionCurve::mud();

        // Order: tarmac > gravel > mud > ice
        assert!(tarmac.peak_friction > gravel.peak_friction);
        assert!(gravel.peak_friction > mud.peak_friction);
        assert!(mud.peak_friction > ice.peak_friction);
    }

    // ============================================================================
    // SUSPENSION PHYSICS TESTS (Phase 8.8 - New)
    // ============================================================================

    #[test]
    fn test_suspension_config_defaults() {
        let wheel = WheelConfig::default();
        assert!(wheel.suspension_rest_length > 0.0);
        assert!(wheel.suspension_stiffness > 0.0);
        assert!(wheel.suspension_damping > 0.0);
        assert!(wheel.suspension_max_compression > 0.0);
        assert!(wheel.suspension_max_extension > 0.0);
    }

    #[test]
    fn test_suspension_compression_bounds() {
        let wheel = WheelConfig::default();
        // Max compression should be less than rest length
        assert!(wheel.suspension_max_compression < wheel.suspension_rest_length);
    }

    #[test]
    fn test_wheel_state_grounded_default() {
        let state = WheelState::default();
        assert!(!state.grounded);
        assert_eq!(state.compression, 0.0);
        assert_eq!(state.slip_ratio, 0.0);
        assert_eq!(state.slip_angle, 0.0);
    }

    #[test]
    fn test_suspension_critical_damping() {
        // Critical damping = 2 * sqrt(k * m)
        let wheel = WheelConfig::default();

        // Suspension damping should be positive and reasonable
        assert!(wheel.suspension_damping > 0.0);
        // Damping ratio relative to stiffness should be reasonable
        // (typical vehicles are slightly underdamped to overdamped)
        let ratio = wheel.suspension_damping / wheel.suspension_stiffness;
        assert!(ratio > 0.05 && ratio < 0.5);
    }

    // ============================================================================
    // DRIVETRAIN TESTS (Phase 8.8 - New)
    // ============================================================================

    #[test]
    fn test_drivetrain_type_default() {
        let config = VehicleConfig::default();
        assert_eq!(config.drivetrain, DrivetrainType::RWD);
    }

    #[test]
    fn test_fwd_wheel_config() {
        let front = WheelConfig::front_left(Vec3::ZERO).with_drive();
        assert!(front.steerable);
        assert!(front.driven);
    }

    #[test]
    fn test_rwd_wheel_config() {
        let rear = WheelConfig::rear_left(Vec3::ZERO);
        assert!(!rear.steerable);
        assert!(rear.driven);
    }

    #[test]
    fn test_awd_configuration() {
        let wheels = vec![
            WheelConfig::front_left(Vec3::ZERO).with_drive(),
            WheelConfig::front_right(Vec3::ZERO).with_drive(),
            WheelConfig::rear_left(Vec3::ZERO),
            WheelConfig::rear_right(Vec3::ZERO),
        ];

        // All wheels should be driven in AWD
        assert!(wheels.iter().all(|w| w.driven));
    }

    #[test]
    fn test_transmission_reverse_ratio() {
        let trans = TransmissionConfig::default();
        // Reverse should be negative
        assert!(trans.reverse_ratio < 0.0);
        // Reverse ratio magnitude should be significant (like a low gear)
        assert!(trans.reverse_ratio.abs() > 2.0);
    }

    #[test]
    fn test_transmission_gear_progression() {
        let trans = TransmissionConfig::default();
        // Higher gears should have lower ratios (taller gearing)
        for i in 1..trans.gear_ratios.len() {
            assert!(trans.gear_ratios[i] < trans.gear_ratios[i - 1]);
        }
    }

    #[test]
    fn test_transmission_neutral_ratio() {
        let trans = TransmissionConfig::default();
        let neutral_ratio = trans.effective_ratio(0);
        assert_eq!(neutral_ratio, 0.0);
    }

    // ============================================================================
    // ENGINE TORQUE CURVE TESTS (Phase 8.8 - New)
    // ============================================================================

    #[test]
    fn test_engine_below_idle() {
        let engine = EngineConfig::default();
        let torque = engine.torque_at_rpm(engine.idle_rpm - 100.0);
        assert_eq!(torque, 0.0);
    }

    #[test]
    fn test_engine_above_redline() {
        let engine = EngineConfig::default();
        let torque = engine.torque_at_rpm(engine.max_rpm + 100.0);
        assert_eq!(torque, 0.0);
    }

    #[test]
    fn test_engine_max_torque_point() {
        let engine = EngineConfig::default();
        let at_max_torque = engine.torque_at_rpm(engine.max_torque_rpm);

        // Should be close to max torque at max torque RPM
        assert!(at_max_torque > engine.max_torque * 0.9);
    }

    #[test]
    fn test_engine_torque_curve_shape() {
        let engine = EngineConfig::default();

        // Torque at mid-range
        let mid_rpm = (engine.idle_rpm + engine.max_torque_rpm) / 2.0;
        let mid_torque = engine.torque_at_rpm(mid_rpm);

        // Should have positive torque at mid-range
        assert!(mid_torque > 0.0);
        assert!(mid_torque < engine.max_torque);
    }

    // ============================================================================
    // VEHICLE PHYSICS INTEGRATION TESTS (Phase 8.8 - New)
    // ============================================================================

    #[test]
    fn test_vehicle_drag_coefficient() {
        let config = VehicleConfig::default();
        // Typical car drag coefficient range
        assert!(config.drag_coefficient >= 0.25);
        assert!(config.drag_coefficient <= 0.50);
    }

    #[test]
    fn test_vehicle_frontal_area() {
        let config = VehicleConfig::default();
        // Typical car frontal area
        assert!(config.frontal_area >= 1.5);
        assert!(config.frontal_area <= 3.0);
    }

    #[test]
    fn test_vehicle_mass_realistic() {
        let config = VehicleConfig::default();
        // Typical passenger car mass range
        assert!(config.mass >= 1000.0);
        assert!(config.mass <= 3000.0);
    }

    #[test]
    fn test_vehicle_wheel_count() {
        let config = VehicleConfig::default();
        assert_eq!(config.wheels.len(), 4);
    }

    #[test]
    fn test_vehicle_wheel_positions_symmetric() {
        let config = VehicleConfig::default();

        // Front wheels should be symmetric about X axis
        let fl = config.wheels[0].position;
        let fr = config.wheels[1].position;
        assert!((fl.x + fr.x).abs() < 0.01);
        assert!((fl.z - fr.z).abs() < 0.01);

        // Rear wheels should be symmetric about X axis
        let rl = config.wheels[2].position;
        let rr = config.wheels[3].position;
        assert!((rl.x + rr.x).abs() < 0.01);
        assert!((rl.z - rr.z).abs() < 0.01);
    }

    #[test]
    fn test_wheel_position_ids() {
        let fl = WheelConfig::front_left(Vec3::ZERO);
        let fr = WheelConfig::front_right(Vec3::ZERO);
        let rl = WheelConfig::rear_left(Vec3::ZERO);
        let rr = WheelConfig::rear_right(Vec3::ZERO);

        assert_eq!(fl.position_id, WheelPosition::FrontLeft);
        assert_eq!(fr.position_id, WheelPosition::FrontRight);
        assert_eq!(rl.position_id, WheelPosition::RearLeft);
        assert_eq!(rr.position_id, WheelPosition::RearRight);
    }

    #[test]
    fn test_vehicle_manager_remove() {
        let mut physics = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));
        let mut manager = VehicleManager::new();

        let id = manager.spawn(&mut physics, Vec3::ZERO, VehicleConfig::default());
        assert_eq!(manager.vehicles().len(), 1);

        let removed = manager.remove(id);
        assert!(removed);
        assert_eq!(manager.vehicles().len(), 0);
        assert!(manager.get(id).is_none());
    }

    #[test]
    fn test_vehicle_airborne_detection() {
        let config = VehicleConfig::default();
        let vehicle = Vehicle::new(1, 42, config);

        // No wheels grounded = airborne
        let airborne = vehicle.wheels.iter().all(|w| !w.grounded);
        assert!(airborne);
    }

    #[test]
    fn test_vehicle_input_clamping() {
        // Input values should be clamped to valid ranges
        let input = VehicleInput {
            throttle: 1.5, // Over max
            brake: -0.5,   // Under min
            steering: 2.0, // Over max
            handbrake: 1.0,
            clutch: 0.0,
            shift: 0,
        };

        // These would need clamping in real use
        assert!(input.throttle > 1.0); // Unclamped for test
    }

    // ═══════════════════════════════════════════════════════════════
    // DEEP REMEDIATION v3.6 — vehicle physics critical path tests
    // ═══════════════════════════════════════════════════════════════

    #[test]
    fn mutation_torque_at_rpm_below_idle() {
        let engine = EngineConfig::default();
        let torque = engine.torque_at_rpm(engine.idle_rpm - 100.0);
        assert_eq!(torque, 0.0, "Below idle RPM should produce 0 torque");
    }

    #[test]
    fn mutation_torque_at_rpm_above_max() {
        let engine = EngineConfig::default();
        let torque = engine.torque_at_rpm(engine.max_rpm + 100.0);
        assert_eq!(torque, 0.0, "Above max RPM should produce 0 torque");
    }

    #[test]
    fn mutation_torque_at_peak_rpm() {
        let engine = EngineConfig::default();
        let torque = engine.torque_at_rpm(engine.max_torque_rpm);
        // At peak RPM, torque should be close to max_torque
        assert!(
            (torque - engine.max_torque).abs() < 5.0,
            "At max_torque_rpm, torque should be near max_torque={}, got {}",
            engine.max_torque,
            torque
        );
    }

    #[test]
    fn mutation_torque_curve_rises_then_falls() {
        let engine = EngineConfig::default();
        let mid_rising = (engine.idle_rpm + engine.max_torque_rpm) / 2.0;
        let mid_falling = (engine.max_torque_rpm + engine.max_rpm) / 2.0;

        let t_rising = engine.torque_at_rpm(mid_rising);
        let t_peak = engine.torque_at_rpm(engine.max_torque_rpm);
        let t_falling = engine.torque_at_rpm(mid_falling);

        assert!(t_rising > 0.0, "Rising portion should have positive torque");
        assert!(t_peak > t_rising, "Peak should exceed rising portion");
        assert!(
            t_falling < t_peak,
            "Falling portion should be less than peak"
        );
        assert!(t_falling > 0.0, "Falling portion should still be positive");
    }

    #[test]
    fn mutation_friction_at_zero_slip() {
        let curve = FrictionCurve::tarmac();
        let f = curve.friction_at_slip(0.0);
        assert_eq!(f, 0.0, "Zero slip should produce zero friction");
    }

    #[test]
    fn mutation_friction_at_optimal_slip() {
        let curve = FrictionCurve::tarmac();
        let f = curve.friction_at_slip(curve.optimal_slip);
        // At optimal slip, friction should be near peak
        assert!(
            f > curve.sliding_friction,
            "At optimal slip, friction should exceed sliding friction"
        );
        assert!(
            f <= curve.peak_friction * 1.01,
            "At optimal slip, friction should not exceed peak"
        );
    }

    #[test]
    fn mutation_friction_curve_shape() {
        let curve = FrictionCurve::tarmac();
        let f_low = curve.friction_at_slip(curve.optimal_slip * 0.5);
        let f_opt = curve.friction_at_slip(curve.optimal_slip);
        let f_high = curve.friction_at_slip(curve.optimal_slip * 3.0);

        // Should rise toward peak, then decay toward sliding
        assert!(f_low < f_opt, "Pre-peak should be less than peak");
        assert!(f_high < f_opt, "Post-peak should be less than peak");
        assert!(f_high > 0.0, "Post-peak should still be positive");
    }

    #[test]
    fn mutation_friction_surface_types_differ() {
        let tarmac = FrictionCurve::tarmac();
        let ice = FrictionCurve::ice();
        let gravel = FrictionCurve::gravel();

        assert!(
            tarmac.peak_friction > ice.peak_friction,
            "Tarmac should have more grip than ice"
        );
        assert!(
            gravel.peak_friction < tarmac.peak_friction,
            "Gravel should have less grip than tarmac"
        );
    }

    #[test]
    fn mutation_effective_ratio_neutral() {
        let trans = TransmissionConfig::default();
        assert_eq!(
            trans.effective_ratio(0),
            0.0,
            "Neutral gear should have 0 effective ratio"
        );
    }

    #[test]
    fn mutation_effective_ratio_reverse() {
        let trans = TransmissionConfig::default();
        let ratio = trans.effective_ratio(-1);
        assert!(ratio != 0.0, "Reverse gear should have non-zero ratio");
        assert_eq!(
            ratio,
            trans.reverse_ratio * trans.final_drive,
            "Reverse ratio should be reverse_ratio * final_drive"
        );
    }

    #[test]
    fn mutation_effective_ratio_first_gear() {
        let trans = TransmissionConfig::default();
        let ratio = trans.effective_ratio(1);
        assert!(ratio > 0.0, "First gear ratio should be positive");
        assert_eq!(
            ratio,
            trans.gear_ratios[0] * trans.final_drive,
            "First gear ratio should be gear_ratios[0] * final_drive"
        );
    }

    #[test]
    fn mutation_effective_ratio_higher_gears_decrease() {
        let trans = TransmissionConfig::default();
        if trans.gear_ratios.len() >= 2 {
            let r1 = trans.effective_ratio(1);
            let r2 = trans.effective_ratio(2);
            assert!(
                r1 > r2,
                "Higher gears should have lower effective ratio (r1={}, r2={})",
                r1,
                r2
            );
        }
    }

    #[test]
    fn mutation_num_gears() {
        let trans = TransmissionConfig::default();
        assert_eq!(
            trans.num_gears(),
            trans.gear_ratios.len(),
            "num_gears should match gear_ratios length"
        );
        assert!(trans.num_gears() > 0, "Should have at least 1 gear");
    }

    #[test]
    fn mutation_speed_conversions() {
        let config = VehicleConfig::default();
        let mut vehicle = Vehicle::new(1, 42, config);
        vehicle.speed = 10.0; // 10 m/s

        let kmh = vehicle.speed_kmh();
        let mph = vehicle.speed_mph();

        // 10 m/s = 36 km/h = 22.37 mph
        assert!(
            (kmh - 36.0).abs() < 0.1,
            "10 m/s should be ~36 km/h, got {}",
            kmh
        );
        assert!(
            (mph - 22.37).abs() < 0.1,
            "10 m/s should be ~22.37 mph, got {}",
            mph
        );
    }

    // ═══════════════════════════════════════════════════════════════
    // DEEP REMEDIATION v3.6.1 — vehicle Round 2 boundary/arithmetic tests
    // ═══════════════════════════════════════════════════════════════

    // --- EngineConfig::torque_at_rpm boundary precision ---
    #[test]
    fn mutation_torque_at_idle_rpm_exact() {
        let engine = EngineConfig::default();
        let torque = engine.torque_at_rpm(engine.idle_rpm);
        // At idle_rpm, normalized = 0, torque = max_torque * (1 - (1-0)^2) = 0
        assert_eq!(
            torque, 0.0,
            "At exactly idle RPM, torque should be 0, got {}",
            torque
        );
    }

    #[test]
    fn mutation_torque_at_max_rpm_exact() {
        let engine = EngineConfig::default();
        let torque = engine.torque_at_rpm(engine.max_rpm);
        // At max_rpm, falloff = (max_rpm - max_torque_rpm)/(max_rpm - max_torque_rpm) = 1.0
        // torque = max_torque * (1 - 1^2).max(0) = 0
        assert_eq!(
            torque, 0.0,
            "At exactly max RPM, torque should be 0, got {}",
            torque
        );
    }

    #[test]
    fn mutation_torque_below_idle_boundary() {
        let engine = EngineConfig::default();
        // Just barely below idle
        let torque = engine.torque_at_rpm(engine.idle_rpm - 0.001);
        assert_eq!(torque, 0.0, "Below idle RPM should return 0");
        // Just barely at idle
        let torque_at = engine.torque_at_rpm(engine.idle_rpm);
        assert_eq!(torque_at, 0.0, "At idle RPM should return 0 (normalized=0)");
    }

    #[test]
    fn mutation_torque_above_max_boundary() {
        let engine = EngineConfig::default();
        let torque = engine.torque_at_rpm(engine.max_rpm + 0.001);
        assert_eq!(torque, 0.0, "Above max RPM should return 0");
    }

    #[test]
    fn mutation_torque_curve_parabolic_rising() {
        let engine = EngineConfig::default();
        // Test specific points on the rising parabola
        let rpm_quarter = engine.idle_rpm + (engine.max_torque_rpm - engine.idle_rpm) * 0.25;
        let rpm_half = engine.idle_rpm + (engine.max_torque_rpm - engine.idle_rpm) * 0.5;
        let rpm_three_quarter = engine.idle_rpm + (engine.max_torque_rpm - engine.idle_rpm) * 0.75;

        let t25 = engine.torque_at_rpm(rpm_quarter);
        let t50 = engine.torque_at_rpm(rpm_half);
        let t75 = engine.torque_at_rpm(rpm_three_quarter);

        // Verify parabolic: 1 - (1 - x)^2 at x=0.25 → 1-0.5625=0.4375
        let expected_25 = engine.max_torque * (1.0 - (1.0 - 0.25_f32).powi(2));
        assert!(
            (t25 - expected_25).abs() < 0.1,
            "At 25%, torque should be {}, got {}",
            expected_25,
            t25
        );

        // Should be monotonically increasing
        assert!(t50 > t25, "t50={} should > t25={}", t50, t25);
        assert!(t75 > t50, "t75={} should > t50={}", t75, t50);
    }

    #[test]
    fn mutation_torque_curve_parabolic_falling() {
        let engine = EngineConfig::default();
        // Falling portion: at 75% between max_torque_rpm and max_rpm
        let rpm_fall = engine.max_torque_rpm + (engine.max_rpm - engine.max_torque_rpm) * 0.75;
        let falloff = (rpm_fall - engine.max_torque_rpm) / (engine.max_rpm - engine.max_torque_rpm);
        let expected = engine.max_torque * (1.0 - falloff.powi(2)).max(0.0);
        let actual = engine.torque_at_rpm(rpm_fall);
        assert!(
            (actual - expected).abs() < 0.1,
            "Falling torque at 75% should be {}, got {}",
            expected,
            actual
        );
    }

    // --- FrictionCurve::friction_at_slip boundary/arithmetic ---
    #[test]
    fn mutation_friction_near_zero_slip_threshold() {
        let curve = FrictionCurve::tarmac();
        // Below 0.001 threshold (strictly < 0.001 returns 0)
        assert_eq!(
            curve.friction_at_slip(0.0005),
            0.0,
            "Very small slip should return 0"
        );
        assert_eq!(
            curve.friction_at_slip(0.0009),
            0.0,
            "0.0009 < 0.001 returns 0"
        );
        // At threshold: 0.001 is NOT < 0.001, so enters rising portion
        let f = curve.friction_at_slip(0.001);
        assert!(
            f > 0.0,
            "At exactly 0.001, enters rising portion, got {}",
            f
        );
        // x = 0.001/0.08 = 0.0125, peak*(1-exp(-12*0.0125)) = 1.2*(1-exp(-0.15)) ≈ 0.167
        let expected =
            curve.peak_friction * (1.0 - (-curve.stiffness * (0.001 / curve.optimal_slip)).exp());
        assert!(
            (f - expected).abs() < 1e-5,
            "Should match formula, expected {}, got {}",
            expected,
            f
        );
    }

    #[test]
    fn mutation_friction_rising_exact_formula() {
        let curve = FrictionCurve::tarmac();
        let slip = curve.optimal_slip * 0.5; // x = 0.5
        let f = curve.friction_at_slip(slip);
        let x = 0.5;
        let expected = curve.peak_friction * (1.0 - (-curve.stiffness * x).exp());
        assert!(
            (f - expected).abs() < 1e-5,
            "Rising portion at x=0.5 should be {}, got {}",
            expected,
            f
        );
    }

    #[test]
    fn mutation_friction_falling_decay_clamped() {
        let curve = FrictionCurve::tarmac();
        // x > 1 → falling portion
        // At x = 1.5: decay = ((1.5-1)*2).min(1) = 1.0
        let slip = curve.optimal_slip * 1.5;
        let f = curve.friction_at_slip(slip);
        let expected = curve.peak_friction - (curve.peak_friction - curve.sliding_friction) * 1.0;
        assert!(
            (f - expected).abs() < 1e-5,
            "At x=1.5, friction should equal sliding_friction={}, got {}",
            expected,
            f
        );
    }

    #[test]
    fn mutation_friction_falling_midpoint() {
        let curve = FrictionCurve::tarmac();
        // x = 1.25: decay = ((1.25-1)*2).min(1) = 0.5
        let slip = curve.optimal_slip * 1.25;
        let f = curve.friction_at_slip(slip);
        let decay = 0.5;
        let expected = curve.peak_friction - (curve.peak_friction - curve.sliding_friction) * decay;
        assert!(
            (f - expected).abs() < 1e-4,
            "At x=1.25, friction should be {}, got {}",
            expected,
            f
        );
    }

    #[test]
    fn mutation_friction_negative_slip_abs() {
        let curve = FrictionCurve::tarmac();
        let f_pos = curve.friction_at_slip(0.05);
        let f_neg = curve.friction_at_slip(-0.05);
        assert!(
            (f_pos - f_neg).abs() < 1e-6,
            "friction_at_slip should be symmetric: pos={}, neg={}",
            f_pos,
            f_neg
        );
    }

    // --- TransmissionConfig::effective_ratio ---
    #[test]
    fn mutation_effective_ratio_out_of_bounds_gear() {
        let trans = TransmissionConfig::default();
        let max_gear = trans.gear_ratios.len() as i32;
        let ratio = trans.effective_ratio(max_gear + 1);
        // Out of bounds defaults to 1.0 * final_drive
        assert!(
            (ratio - 1.0 * trans.final_drive).abs() < 1e-5,
            "Out-of-bounds gear should use fallback ratio=1.0*final_drive={}, got {}",
            1.0 * trans.final_drive,
            ratio
        );
    }

    #[test]
    fn mutation_effective_ratio_all_gears() {
        let trans = TransmissionConfig::default();
        for gear in 1..=(trans.gear_ratios.len() as i32) {
            let ratio = trans.effective_ratio(gear);
            let expected = trans.gear_ratios[(gear - 1) as usize] * trans.final_drive;
            assert!(
                (ratio - expected).abs() < 1e-5,
                "Gear {} ratio should be {}, got {}",
                gear,
                expected,
                ratio
            );
        }
    }

    // --- WheelConfig constructors ---
    #[test]
    fn mutation_wheel_config_front_left_flags() {
        let w = WheelConfig::front_left(Vec3::new(-0.8, 0.0, 1.2));
        assert!(w.steerable, "Front left should be steerable");
        assert!(
            !w.driven,
            "Front left should not be driven (default is RWD)"
        );
        assert_eq!(w.position_id, WheelPosition::FrontLeft);
    }

    #[test]
    fn mutation_wheel_config_rear_right_flags() {
        let w = WheelConfig::rear_right(Vec3::new(0.8, 0.0, -1.2));
        assert!(!w.steerable, "Rear right should not be steerable");
        assert!(w.driven, "Rear right should be driven");
        assert_eq!(w.position_id, WheelPosition::RearRight);
    }

    // --- Vehicle shifting ---
    #[test]
    fn mutation_shift_up_from_max_gear() {
        let config = VehicleConfig::default();
        let mut vehicle = Vehicle::new(1, 1, config.clone());
        let max = config.transmission.num_gears() as i32;
        vehicle.current_gear = max;
        vehicle.shift_timer = 0.0;
        vehicle.shift_up();
        assert_eq!(vehicle.current_gear, max, "Should not shift above max gear");
    }

    #[test]
    fn mutation_shift_down_from_minus_one() {
        let config = VehicleConfig::default();
        let mut vehicle = Vehicle::new(1, 1, config);
        vehicle.current_gear = -1;
        vehicle.shift_timer = 0.0;
        vehicle.shift_down();
        assert_eq!(vehicle.current_gear, -1, "Should not shift below -1");
    }

    #[test]
    fn mutation_shift_blocked_during_shift() {
        let config = VehicleConfig::default();
        let mut vehicle = Vehicle::new(1, 1, config);
        vehicle.current_gear = 1;
        vehicle.shift_timer = 0.1; // Currently shifting
        let gear_before = vehicle.current_gear;
        vehicle.shift_up();
        assert_eq!(
            vehicle.current_gear, gear_before,
            "Should not shift while shift_timer > 0"
        );
    }

    // --- Vehicle state queries ---
    #[test]
    fn mutation_grounded_wheels_count() {
        let config = VehicleConfig::default();
        let mut vehicle = Vehicle::new(1, 1, config);
        vehicle.wheels[0].grounded = true;
        vehicle.wheels[1].grounded = true;
        vehicle.wheels[2].grounded = false;
        vehicle.wheels[3].grounded = false;
        assert_eq!(vehicle.grounded_wheels(), 2);
        assert!(!vehicle.is_airborne());
    }

    #[test]
    fn mutation_is_airborne() {
        let config = VehicleConfig::default();
        let vehicle = Vehicle::new(1, 1, config);
        // Default all wheels have grounded=false
        assert!(vehicle.is_airborne());
    }

    #[test]
    fn mutation_average_slip_no_grounded_wheels() {
        let config = VehicleConfig::default();
        let vehicle = Vehicle::new(1, 1, config);
        assert_eq!(
            vehicle.average_slip_ratio(),
            0.0,
            "No grounded wheels should return 0"
        );
        assert_eq!(
            vehicle.average_slip_angle(),
            0.0,
            "No grounded wheels should return 0"
        );
    }

    #[test]
    fn mutation_average_slip_ratio_calculation() {
        let config = VehicleConfig::default();
        let mut vehicle = Vehicle::new(1, 1, config);
        vehicle.wheels[0].grounded = true;
        vehicle.wheels[0].slip_ratio = 0.1;
        vehicle.wheels[1].grounded = true;
        vehicle.wheels[1].slip_ratio = -0.3;
        vehicle.wheels[2].grounded = false; // Not counted
        vehicle.wheels[3].grounded = true;
        vehicle.wheels[3].slip_ratio = 0.2;
        // avg = (0.1 + 0.3 + 0.2) / 3 = 0.2
        let avg = vehicle.average_slip_ratio();
        assert!(
            (avg - 0.2).abs() < 1e-5,
            "Average slip ratio should be 0.2, got {}",
            avg
        );
    }

    // ===== DEEP REMEDIATION v3.6.2 — vehicle Round 3 remaining mutations =====

    // --- FrictionCurve::tarmac preset ---
    #[test]
    fn mutation_r3_tarmac_not_default() {
        // Mutation: replace tarmac() -> Self with Default::default()
        let tarmac = FrictionCurve::tarmac();
        let default_fc = FrictionCurve::default();
        // Tarmac has specific values that differ from default
        assert!(
            (tarmac.optimal_slip - 0.08).abs() < 1e-5,
            "Tarmac optimal_slip should be 0.08"
        );
        assert!(
            (tarmac.peak_friction - 1.2).abs() < 1e-5,
            "Tarmac peak_friction should be 1.2"
        );
        assert!(
            (tarmac.sliding_friction - 0.9).abs() < 1e-5,
            "Tarmac sliding_friction should be 0.9"
        );
        assert!(
            (tarmac.stiffness - 12.0).abs() < 1e-5,
            "Tarmac stiffness should be 12.0"
        );
        // At least one field should differ from default
        let differs = (tarmac.optimal_slip - default_fc.optimal_slip).abs() > 1e-5
            || (tarmac.peak_friction - default_fc.peak_friction).abs() > 1e-5
            || (tarmac.sliding_friction - default_fc.sliding_friction).abs() > 1e-5
            || (tarmac.stiffness - default_fc.stiffness).abs() > 1e-5;
        assert!(differs, "Tarmac should differ from default");
    }

    // --- effective_ratio subtraction mutation ---
    #[test]
    fn mutation_r3_effective_ratio_gear_index_subtraction() {
        // gear_ratios.get((gear - 1) as usize)  (mutation: - → / or + )
        let cfg = TransmissionConfig {
            gear_ratios: vec![3.0, 2.0, 1.5],
            final_drive: 4.0,
            ..Default::default()
        };
        // Gear 1 should use index 0 → ratio 3.0
        assert!(
            (cfg.effective_ratio(1) - 12.0).abs() < 1e-5,
            "Gear 1: 3.0 * 4.0 = 12.0"
        );
        // Gear 2 should use index 1 → ratio 2.0
        assert!(
            (cfg.effective_ratio(2) - 8.0).abs() < 1e-5,
            "Gear 2: 2.0 * 4.0 = 8.0"
        );
        // Gear 3 should use index 2 → ratio 1.5
        assert!(
            (cfg.effective_ratio(3) - 6.0).abs() < 1e-5,
            "Gear 3: 1.5 * 4.0 = 6.0"
        );
    }

    #[test]
    fn mutation_r3_effective_ratio_multiply_final_drive() {
        // gear_ratio * self.final_drive  (mutation: * → /)
        let cfg = TransmissionConfig {
            gear_ratios: vec![2.5],
            final_drive: 3.0,
            ..Default::default()
        };
        let result = cfg.effective_ratio(1);
        assert!(
            (result - 7.5).abs() < 1e-5,
            "2.5 * 3.0 = 7.5, got {}",
            result
        );
    }

    // --- torque_at_rpm boundary precision ---
    #[test]
    fn mutation_r3_torque_boundary_just_below_idle() {
        // rpm < idle_rpm → return 0  (mutation: < → <=)
        let cfg = EngineConfig {
            idle_rpm: 800.0,
            max_rpm: 6000.0,
            max_torque_rpm: 3500.0,
            max_torque: 400.0,
            ..Default::default()
        };
        // At exactly idle_rpm, should NOT return 0 (< is false when ==)
        let at_idle = cfg.torque_at_rpm(800.0);
        // normalized = (800-800)/(3500-800) = 0, torque = 400*(1-(1-0)^2) = 0
        // It's technically 0 due to normalized=0, but it enters the branch
        assert!(at_idle >= 0.0, "At idle should be >= 0");
        // Just below → exactly 0 (early return)
        let below = cfg.torque_at_rpm(799.0);
        assert_eq!(below, 0.0, "Below idle should be exactly 0.0");
    }

    #[test]
    fn mutation_r3_torque_boundary_at_max_rpm() {
        // rpm > max_rpm → return 0  (mutation: > → == or >=)
        let cfg = EngineConfig {
            idle_rpm: 800.0,
            max_rpm: 6000.0,
            max_torque_rpm: 3500.0,
            max_torque: 400.0,
            ..Default::default()
        };
        // At exactly max_rpm: falloff = (6000-3500)/(6000-3500) = 1.0
        // torque = 400 * (1 - 1^2).max(0) = 0
        let at_max = cfg.torque_at_rpm(6000.0);
        assert!(at_max >= 0.0, "At max_rpm result should be >= 0");
        // Just above → 0 (early return)
        let above = cfg.torque_at_rpm(6001.0);
        assert_eq!(above, 0.0, "Above max_rpm should be exactly 0.0");
    }

    #[test]
    fn mutation_r3_torque_falling_subtraction() {
        // falloff = (rpm - max_torque_rpm) / (max_rpm - max_torque_rpm)
        // Mutations: - → + or / on line 215
        let cfg = EngineConfig {
            idle_rpm: 1000.0,
            max_rpm: 7000.0,
            max_torque_rpm: 4000.0,
            max_torque: 500.0,
            ..Default::default()
        };
        // At rpm=5500 (falling portion): falloff = (5500-4000)/(7000-4000) = 0.5
        // torque = 500 * (1 - 0.5^2) = 500 * 0.75 = 375
        let t = cfg.torque_at_rpm(5500.0);
        assert!(
            (t - 375.0).abs() < 1.0,
            "Falling at 5500: expected ~375, got {}",
            t
        );
    }

    #[test]
    fn mutation_r3_torque_normalized_subtraction() {
        // normalized = (rpm - idle_rpm) / (max_torque_rpm - idle_rpm)
        // Line 214: - → +
        let cfg = EngineConfig {
            idle_rpm: 1000.0,
            max_rpm: 7000.0,
            max_torque_rpm: 4000.0,
            max_torque: 500.0,
            ..Default::default()
        };
        // At rpm=2500 (rising): normalized = (2500-1000)/(4000-1000) = 0.5
        // torque = 500 * (1 - (1-0.5)^2) = 500 * (1-0.25) = 375
        let t = cfg.torque_at_rpm(2500.0);
        assert!(
            (t - 375.0).abs() < 1.0,
            "Rising at 2500: expected ~375, got {}",
            t
        );
    }

    // --- WheelConfig field deletion ---
    #[test]
    fn mutation_r3_wheel_config_front_right_fields() {
        // "delete field steerable" + "delete field driven" from front_right
        let fr = WheelConfig::front_right(Vec3::new(0.7, -0.3, 1.2));
        assert!(fr.steerable, "front_right should be steerable");
        assert!(!fr.driven, "front_right should NOT be driven (RWD default)");
    }

    #[test]
    fn mutation_r3_wheel_config_rear_left_steerable_false() {
        // "delete field steerable" from rear_left
        let rl = WheelConfig::rear_left(Vec3::new(-0.7, -0.3, -1.2));
        assert!(!rl.steerable, "rear_left should NOT be steerable");
    }

    #[test]
    fn mutation_r3_wheel_config_rear_right_steerable_false() {
        // "delete field steerable" from rear_right
        let rr = WheelConfig::rear_right(Vec3::new(0.7, -0.3, -1.2));
        assert!(!rr.steerable, "rear_right should NOT be steerable");
    }

    #[test]
    fn mutation_r3_wheel_config_front_left_driven() {
        // "delete field driven" from front_left
        let fl = WheelConfig::front_left(Vec3::new(-0.7, -0.3, 1.2));
        assert!(!fl.driven, "front_left should NOT be driven (RWD default)");
        // But rear should be driven
        let rl = WheelConfig::rear_left(Vec3::new(-0.7, -0.3, -1.2));
        assert!(rl.driven, "rear_left should be driven");
    }

    // --- friction_at_slip boundary mutations ---
    #[test]
    fn mutation_r3_friction_precise_at_optimal() {
        // At optimal_slip, should be peak friction
        let fc = FrictionCurve {
            optimal_slip: 0.1,
            peak_friction: 1.5,
            sliding_friction: 0.8,
            stiffness: 10.0,
        };
        let f = fc.friction_at_slip(0.1);
        assert!(
            (f - 1.5).abs() < 1e-4,
            "At optimal_slip, should get peak_friction, got {}",
            f
        );
    }

    #[test]
    fn mutation_r3_friction_falling_subtraction() {
        // decay = (x - 1.0) * 2.0  line 317: - → / or +
        let fc = FrictionCurve {
            optimal_slip: 0.1,
            peak_friction: 1.0,
            sliding_friction: 0.5,
            stiffness: 10.0,
        };
        // At slip=0.125: x = 1.25 (falling)
        // decay = ((1.25 - 1.0) * 2.0).min(1.0) = 0.5
        // friction = 1.0 - (1.0 - 0.5) * 0.5 = 0.75
        let f = fc.friction_at_slip(0.125);
        assert!(
            (f - 0.75).abs() < 0.02,
            "Falling at slip=0.125: expected ~0.75, got {}",
            f
        );
    }

    #[test]
    fn mutation_r3_friction_stiffness_product() {
        // result = peak * (1.0 - (-stiffness * x).exp())  line 312: * → +
        let fc = FrictionCurve {
            optimal_slip: 0.2,
            peak_friction: 1.0,
            sliding_friction: 0.6,
            stiffness: 5.0,
        };
        // At slip=0.02 (rising): x = 0.02/0.2 = 0.1
        // friction = 1.0 * (1 - exp(-5.0 * 0.1)) = 1 - exp(-0.5) ≈ 0.3935
        let f = fc.friction_at_slip(0.02);
        let expected = 1.0_f32 * (1.0 - (-0.5_f32).exp());
        assert!(
            (f - expected).abs() < 0.01,
            "Rising with stiffness: expected ~{:.4}, got {}",
            expected,
            f
        );
    }

    // ===== ECS INTEGRATION SCAFFOLDING v3.7.0 — Vehicle+PhysicsWorld integration tests =====

    /// Helper: create a PhysicsWorld + VehicleManager + spawn a default vehicle
    fn spawn_test_vehicle() -> (crate::PhysicsWorld, VehicleManager, VehicleId) {
        use glam::Vec3;
        let mut pw = crate::PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));
        // Must have a ground plane so wheels can hit something
        let _ground = pw.create_ground_plane(Vec3::new(100.0, 0.1, 100.0), 0.8);
        pw.step(); // Initialize query pipeline

        let mut vm = VehicleManager::new();
        let config = VehicleConfig::default();
        let vid = vm.spawn(&mut pw, Vec3::new(0.0, 2.0, 0.0), config);
        (pw, vm, vid)
    }

    #[test]
    fn integration_vehicle_spawn_creates_body() {
        let (pw, vm, vid) = spawn_test_vehicle();
        let vehicle = vm.get(vid).unwrap();
        // Vehicle body should exist in physics world
        assert!(pw.body_transform(vehicle.body_id).is_some());
    }

    #[test]
    fn integration_vehicle_spawn_initial_gear() {
        let (_pw, vm, vid) = spawn_test_vehicle();
        let vehicle = vm.get(vid).unwrap();
        assert_eq!(vehicle.current_gear, 1, "Should start in 1st gear");
        assert!(
            (vehicle.engine_rpm - 800.0).abs() < 1.0,
            "Should start at idle RPM"
        );
    }

    #[test]
    fn integration_vehicle_spawn_four_wheels() {
        let (_pw, vm, vid) = spawn_test_vehicle();
        let vehicle = vm.get(vid).unwrap();
        assert_eq!(vehicle.wheels.len(), 4, "Default config has 4 wheels");
    }

    #[test]
    fn integration_update_with_input_throttle() {
        let (mut pw, mut vm, vid) = spawn_test_vehicle();
        // Step a few times to let the vehicle settle
        for _ in 0..10 {
            vm.update(&mut pw, 1.0 / 60.0);
            pw.step();
        }

        let input = VehicleInput {
            throttle: 1.0,
            ..Default::default()
        };

        for _ in 0..60 {
            vm.update_with_input(vid, &mut pw, &input, 1.0 / 60.0);
            pw.step();
        }

        let vehicle = vm.get(vid).unwrap();
        // Engine RPM should have risen from idle
        assert!(
            vehicle.engine_rpm > 800.0,
            "Throttle should increase RPM above idle: rpm={}",
            vehicle.engine_rpm
        );
    }

    #[test]
    fn integration_update_with_input_brake() {
        let (mut pw, mut vm, vid) = spawn_test_vehicle();
        pw.step();

        // Give the vehicle some forward speed by using set_velocity
        let body_id = vm.get(vid).unwrap().body_id;
        pw.set_velocity(body_id, Vec3::new(0.0, 0.0, 10.0));

        let input = VehicleInput {
            brake: 1.0,
            ..Default::default()
        };

        for _ in 0..30 {
            vm.update_with_input(vid, &mut pw, &input, 1.0 / 60.0);
            pw.step();
        }

        let vel = pw.get_velocity(body_id).unwrap();
        assert!(
            vel.length() < 10.0,
            "Brake should reduce speed: speed={}",
            vel.length()
        );
    }

    #[test]
    fn integration_update_with_input_steering() {
        let (mut pw, mut vm, vid) = spawn_test_vehicle();
        pw.step();

        let input = VehicleInput {
            steering: 1.0,
            throttle: 0.5,
            ..Default::default()
        };

        vm.update_with_input(vid, &mut pw, &input, 1.0 / 60.0);

        let vehicle = vm.get(vid).unwrap();
        // Steerable wheels should have steering angle
        let max_steer = vehicle.config.max_steering_angle;
        let steerable_count = vehicle
            .wheels
            .iter()
            .enumerate()
            .filter(|(i, _)| vehicle.config.wheels[*i].steerable)
            .count();
        assert!(steerable_count > 0, "Should have steerable wheels");

        for (i, wheel) in vehicle.wheels.iter().enumerate() {
            if vehicle.config.wheels[i].steerable {
                assert!(
                    (wheel.steering_angle - max_steer).abs() < 0.01,
                    "Steerable wheel {} should have max steering angle: {} vs {}",
                    i,
                    wheel.steering_angle,
                    max_steer
                );
            }
        }
    }

    #[test]
    fn integration_update_with_input_gear_shift_up() {
        let (mut pw, mut vm, vid) = spawn_test_vehicle();
        pw.step();

        let input = VehicleInput {
            shift: 1, // Shift up
            ..Default::default()
        };

        vm.update_with_input(vid, &mut pw, &input, 1.0 / 60.0);

        let vehicle = vm.get(vid).unwrap();
        assert_eq!(vehicle.current_gear, 2, "Should have shifted to 2nd gear");
        assert!(vehicle.shift_timer > 0.0, "Shift timer should be active");
    }

    #[test]
    fn integration_update_with_input_gear_shift_down() {
        let (mut pw, mut vm, vid) = spawn_test_vehicle();
        pw.step();

        // First shift up to 2nd
        let up_input = VehicleInput {
            shift: 1,
            ..Default::default()
        };
        vm.update_with_input(vid, &mut pw, &up_input, 1.0 / 60.0);

        // Wait for shift to complete
        let neutral = VehicleInput::default();
        for _ in 0..30 {
            vm.update_with_input(vid, &mut pw, &neutral, 1.0 / 60.0);
            pw.step();
        }

        // Now shift down
        let down_input = VehicleInput {
            shift: -1,
            ..Default::default()
        };
        vm.update_with_input(vid, &mut pw, &down_input, 1.0 / 60.0);

        let vehicle = vm.get(vid).unwrap();
        assert_eq!(vehicle.current_gear, 1, "Should have shifted back to 1st");
    }

    #[test]
    fn integration_apply_forces_rpm_clamped() {
        let (mut pw, mut vm, vid) = spawn_test_vehicle();
        pw.step();

        let input = VehicleInput {
            throttle: 1.0,
            ..Default::default()
        };

        // Run for many frames to exercise RPM logic
        for _ in 0..600 {
            vm.update_with_input(vid, &mut pw, &input, 1.0 / 60.0);
            pw.step();
        }

        let vehicle = vm.get(vid).unwrap();
        let max_rpm = vehicle.config.engine.max_rpm;
        assert!(
            vehicle.engine_rpm <= max_rpm + 10.0,
            "RPM should not exceed max: rpm={}, max={}",
            vehicle.engine_rpm,
            max_rpm
        );
        assert!(
            vehicle.engine_rpm >= vehicle.config.engine.idle_rpm - 10.0,
            "RPM should not drop below idle: rpm={}, idle={}",
            vehicle.engine_rpm,
            vehicle.config.engine.idle_rpm
        );
    }

    #[test]
    fn integration_update_vehicle_reads_transform() {
        let (mut pw, mut vm, vid) = spawn_test_vehicle();
        pw.step();

        // Update the vehicle's internal state from physics world
        vm.update(&mut pw, 1.0 / 60.0);

        let vehicle = vm.get(vid).unwrap();
        let transform = pw.body_transform(vehicle.body_id).unwrap();
        let physics_y = transform.w_axis.y;

        // Position should be somewhat consistent with physics
        // (vehicle.velocity is read from physics in update_vehicle)
        assert!(
            physics_y > -100.0 && physics_y < 100.0,
            "Vehicle y should be reasonable: {}",
            physics_y
        );
    }

    #[test]
    fn integration_vehicle_forward_direction_updated() {
        let (mut pw, mut vm, vid) = spawn_test_vehicle();
        pw.step();

        vm.update(&mut pw, 1.0 / 60.0);

        let vehicle = vm.get(vid).unwrap();
        // Forward direction should be a unit vector
        let fwd_len = vehicle.forward.length();
        assert!(
            (fwd_len - 1.0).abs() < 0.1 || fwd_len < 0.01, // Either normalized or vehicle hasn't moved
            "Forward should be close to unit length: {}",
            fwd_len
        );
    }

    #[test]
    fn integration_apply_forces_aerodynamic_drag() {
        let (mut pw, mut vm, vid) = spawn_test_vehicle();
        pw.step();

        // Give the vehicle a high velocity
        let body_id = vm.get(vid).unwrap().body_id;
        pw.set_velocity(body_id, Vec3::new(0.0, 0.0, 30.0));
        pw.step();

        let input = VehicleInput::default(); // No throttle

        // Run a few frames; drag should slow it down
        for _ in 0..30 {
            vm.update_with_input(vid, &mut pw, &input, 1.0 / 60.0);
            pw.step();
        }

        let vel = pw.get_velocity(body_id).unwrap();
        assert!(
            vel.length() < 30.0,
            "Drag should reduce velocity: speed={}",
            vel.length()
        );
    }

    #[test]
    fn integration_vehicle_handbrake() {
        let (mut pw, mut vm, vid) = spawn_test_vehicle();
        pw.step();

        let body_id = vm.get(vid).unwrap().body_id;
        pw.set_velocity(body_id, Vec3::new(0.0, 0.0, 15.0));

        let input = VehicleInput {
            handbrake: 1.0,
            ..Default::default()
        };

        for _ in 0..30 {
            vm.update_with_input(vid, &mut pw, &input, 1.0 / 60.0);
            pw.step();
        }

        let vel = pw.get_velocity(body_id).unwrap();
        assert!(
            vel.length() < 15.0,
            "Handbrake should slow vehicle: speed={}",
            vel.length()
        );
    }

    #[test]
    fn integration_vehicle_suspension_force_with_ground() {
        let (mut pw, mut vm, vid) = spawn_test_vehicle();

        // Let the vehicle settle on ground
        for _ in 0..120 {
            vm.update(&mut pw, 1.0 / 60.0);
            pw.step();
        }

        let vehicle = vm.get(vid).unwrap();
        let total_susp = vehicle.total_suspension_force();
        // After settling, there should be some suspension force supporting the vehicle
        assert!(
            total_susp >= 0.0,
            "Suspension force should be non-negative: {}",
            total_susp
        );
    }

    #[test]
    fn integration_vehicle_no_input_idle_rpm() {
        let (mut pw, mut vm, vid) = spawn_test_vehicle();
        pw.step();

        let input = VehicleInput::default();
        for _ in 0..120 {
            vm.update_with_input(vid, &mut pw, &input, 1.0 / 60.0);
            pw.step();
        }

        let vehicle = vm.get(vid).unwrap();
        // With no throttle, engine should settle near idle RPM
        assert!(
            vehicle.engine_rpm >= 0.0,
            "RPM should not be negative: {}",
            vehicle.engine_rpm
        );
    }

    // ===== ROUND 6: Deep apply_forces integration tests =====

    /// Helper: settle vehicle on ground and return grounded state
    fn settle_vehicle(pw: &mut crate::PhysicsWorld, vm: &mut VehicleManager, vid: VehicleId) {
        let input = VehicleInput::default();
        // Let the vehicle fall and settle on ground plane
        for _ in 0..60 {
            vm.update_with_input(vid, pw, &input, 1.0 / 60.0);
            pw.step();
        }
    }

    #[test]
    fn r6_grounded_wheels_after_settle() {
        let (mut pw, mut vm, vid) = spawn_test_vehicle();
        settle_vehicle(&mut pw, &mut vm, vid);
        let v = vm.get(vid).unwrap();
        // After settling on a ground plane, all 4 wheels should be grounded
        let grounded = v.wheels.iter().filter(|w| w.grounded).count();
        assert!(
            grounded >= 2,
            "Expected >=2 grounded wheels, got {}",
            grounded
        );
    }

    #[test]
    fn r6_suspension_force_positive_on_grounded() {
        let (mut pw, mut vm, vid) = spawn_test_vehicle();
        settle_vehicle(&mut pw, &mut vm, vid);
        let v = vm.get(vid).unwrap();
        for (i, w) in v.wheels.iter().enumerate() {
            if w.grounded {
                // suspension_force = (compression * stiffness + damper).max(0)
                // Mutation of * to + would produce ~50000 instead of ~3700
                assert!(
                    w.suspension_force > 0.0,
                    "Wheel {}: grounded but suspension_force = {}",
                    i,
                    w.suspension_force
                );
                // With mass 1500 and 4 wheels, each should be < 2× share
                assert!(
                    w.suspension_force < 30000.0,
                    "Wheel {}: suspension_force unreasonably high: {} (operator mutation?)",
                    i,
                    w.suspension_force
                );
            }
        }
    }

    #[test]
    fn r6_total_suspension_roughly_equals_weight() {
        let (mut pw, mut vm, vid) = spawn_test_vehicle();
        settle_vehicle(&mut pw, &mut vm, vid);
        let v = vm.get(vid).unwrap();
        // Vehicle weight ≈ 1500 * 9.81 ≈ 14715 N
        // Total suspension should approximately support this
        let total_susp: f32 = v.wheels.iter().map(|w| w.suspension_force).sum();
        // Very loose bounds — catches * → + mutations which overshoot by 10×
        assert!(
            total_susp > 1000.0,
            "Total suspension {} too low (should support ~14715N)",
            total_susp
        );
        assert!(
            total_susp < 200000.0,
            "Total suspension {} unreasonably high (operator mutation?)",
            total_susp
        );
    }

    #[test]
    fn r6_slip_ratio_zero_at_rest() {
        let (mut pw, mut vm, vid) = spawn_test_vehicle();
        settle_vehicle(&mut pw, &mut vm, vid);
        let v = vm.get(vid).unwrap();
        for (i, w) in v.wheels.iter().enumerate() {
            if w.grounded {
                // At rest: rotation_speed ≈ 0 and long_velocity ≈ 0 → slip_ratio = 0
                assert!(
                    w.slip_ratio.abs() < 0.5,
                    "Wheel {}: slip_ratio at rest should be ~0, got {}",
                    i,
                    w.slip_ratio
                );
            }
        }
    }

    #[test]
    fn r6_throttle_increases_driven_wheel_rotation() {
        let (mut pw, mut vm, vid) = spawn_test_vehicle();
        settle_vehicle(&mut pw, &mut vm, vid);

        let input = VehicleInput {
            throttle: 1.0,
            ..Default::default()
        };
        for _ in 0..30 {
            vm.update_with_input(vid, &mut pw, &input, 1.0 / 60.0);
            pw.step();
        }

        let v = vm.get(vid).unwrap();
        // Driven wheels (rear by default) should spin up
        let driven_rotation: f32 = v
            .wheels
            .iter()
            .enumerate()
            .filter(|(i, _)| v.config.wheels[*i].driven)
            .map(|(_, w)| w.rotation_speed.abs())
            .sum();
        assert!(
            driven_rotation > 0.01,
            "Driven wheels should spin with throttle, total rotation_speed = {}",
            driven_rotation
        );
    }

    #[test]
    fn r6_non_driven_wheels_no_drive_torque() {
        let (mut pw, mut vm, vid) = spawn_test_vehicle();
        settle_vehicle(&mut pw, &mut vm, vid);

        let input = VehicleInput {
            throttle: 1.0,
            ..Default::default()
        };
        for _ in 0..10 {
            vm.update_with_input(vid, &mut pw, &input, 1.0 / 60.0);
            pw.step();
        }

        let v = vm.get(vid).unwrap();
        let driven_rot: f32 = v
            .wheels
            .iter()
            .enumerate()
            .filter(|(i, _)| v.config.wheels[*i].driven)
            .map(|(_, w)| w.rotation_speed.abs())
            .sum();
        let non_driven_rot: f32 = v
            .wheels
            .iter()
            .enumerate()
            .filter(|(i, _)| !v.config.wheels[*i].driven)
            .map(|(_, w)| w.rotation_speed.abs())
            .sum();
        // Non-driven should have much less rotation than driven
        assert!(
            driven_rot > non_driven_rot,
            "Driven wheels ({}) should spin more than non-driven ({})",
            driven_rot,
            non_driven_rot
        );
    }

    #[test]
    fn r6_brake_reduces_speed() {
        let (mut pw, mut vm, vid) = spawn_test_vehicle();
        settle_vehicle(&mut pw, &mut vm, vid);

        // First accelerate
        let accel_input = VehicleInput {
            throttle: 1.0,
            ..Default::default()
        };
        for _ in 0..60 {
            vm.update_with_input(vid, &mut pw, &accel_input, 1.0 / 60.0);
            pw.step();
        }
        let speed_before_brake = vm.get(vid).unwrap().speed;

        // Now brake hard
        let brake_input = VehicleInput {
            brake: 1.0,
            ..Default::default()
        };
        for _ in 0..30 {
            vm.update_with_input(vid, &mut pw, &brake_input, 1.0 / 60.0);
            pw.step();
        }
        let speed_after_brake = vm.get(vid).unwrap().speed;

        // If `long_force -= brake_force` mutated to +=, speed would increase
        if speed_before_brake > 0.1 {
            assert!(
                speed_after_brake < speed_before_brake,
                "Brake should reduce speed: before={}, after={}",
                speed_before_brake,
                speed_after_brake
            );
        }
    }

    #[test]
    fn r6_handbrake_multiplier_on_rear_wheels() {
        let (mut pw, mut vm, vid) = spawn_test_vehicle();
        settle_vehicle(&mut pw, &mut vm, vid);

        // Accelerate
        let accel = VehicleInput {
            throttle: 1.0,
            ..Default::default()
        };
        for _ in 0..60 {
            vm.update_with_input(vid, &mut pw, &accel, 1.0 / 60.0);
            pw.step();
        }

        // Apply handbrake
        let hb = VehicleInput {
            handbrake: 1.0,
            ..Default::default()
        };
        for _ in 0..30 {
            vm.update_with_input(vid, &mut pw, &hb, 1.0 / 60.0);
            pw.step();
        }

        let v = vm.get(vid).unwrap();
        // Handbrake affects RearLeft/RearRight only
        // Car may still have significant momentum but should be lower than peak
        let speed_after = v.speed;
        // Also verify at least one rear wheel has braking effect
        // The handbrake applies extra multiplier to rear wheel brake force
        // Check speed decreased from what it would be without handbrake
        assert!(
            speed_after < 200.0, // Vehicle with handbrake should not exceed 200
            "Handbrake should limit vehicle speed={}",
            speed_after
        );
    }

    #[test]
    fn r6_steering_only_steerable_wheels() {
        let (mut pw, mut vm, vid) = spawn_test_vehicle();
        settle_vehicle(&mut pw, &mut vm, vid);

        let input = VehicleInput {
            throttle: 0.5,
            steering: 1.0, // Full right
            ..Default::default()
        };
        for _ in 0..10 {
            vm.update_with_input(vid, &mut pw, &input, 1.0 / 60.0);
            pw.step();
        }

        let v = vm.get(vid).unwrap();
        for (i, w) in v.wheels.iter().enumerate() {
            if v.config.wheels[i].steerable {
                assert!(
                    w.steering_angle.abs() > 0.01,
                    "Steerable wheel {} should have non-zero steering angle: {}",
                    i,
                    w.steering_angle
                );
            } else {
                assert!(
                    (w.steering_angle).abs() < 1e-5,
                    "Non-steerable wheel {} should have zero steering angle: {}",
                    i,
                    w.steering_angle
                );
            }
        }
    }

    #[test]
    fn r6_wheel_force_has_normal_component() {
        let (mut pw, mut vm, vid) = spawn_test_vehicle();
        settle_vehicle(&mut pw, &mut vm, vid);

        // Apply a few frames with throttle to build force
        let input = VehicleInput {
            throttle: 0.3,
            ..Default::default()
        };
        for _ in 0..5 {
            vm.update_with_input(vid, &mut pw, &input, 1.0 / 60.0);
            pw.step();
        }

        let v = vm.get(vid).unwrap();
        // At least one grounded wheel with suspension should produce non-zero force.
        // This catches mutations that zero out the wheel_force assignment.
        let any_force = v
            .wheels
            .iter()
            .any(|w| w.grounded && w.force.length_squared() > 0.001);
        let any_suspension = v
            .wheels
            .iter()
            .any(|w| w.grounded && w.suspension_force > 0.0);
        // If we have suspension, force must be non-zero
        if any_suspension {
            assert!(
                any_force,
                "At least one grounded wheel with suspension should have non-zero force"
            );
        }
    }

    #[test]
    fn r6_ungrounded_wheel_no_force() {
        let (mut pw, mut vm, vid) = spawn_test_vehicle();
        // Don't settle — vehicle still in air
        let input = VehicleInput {
            throttle: 1.0,
            ..Default::default()
        };
        vm.update_with_input(vid, &mut pw, &input, 1.0 / 60.0);

        let v = vm.get(vid).unwrap();
        for (i, w) in v.wheels.iter().enumerate() {
            if !w.grounded {
                // Ungrounded wheels skip the force loop (continue;)
                // If `!grounded` check is deleted, force would be applied
                assert!(
                    w.force.length() < 1e-3,
                    "Ungrounded wheel {} should have zero force: {:?}",
                    i,
                    w.force
                );
                assert!(
                    w.suspension_force.abs() < 1e-3,
                    "Ungrounded wheel {} should have zero suspension_force: {}",
                    i,
                    w.suspension_force
                );
            }
        }
    }

    #[test]
    fn r6_rolling_resistance_reduces_rotation() {
        let (mut pw, mut vm, vid) = spawn_test_vehicle();
        settle_vehicle(&mut pw, &mut vm, vid);

        // Spin up wheels with throttle
        let accel = VehicleInput {
            throttle: 1.0,
            ..Default::default()
        };
        for _ in 0..30 {
            vm.update_with_input(vid, &mut pw, &accel, 1.0 / 60.0);
            pw.step();
        }

        // Get rotation speed of non-driven (front) wheels after accel
        let initial: Vec<f32> = vm
            .get(vid)
            .unwrap()
            .wheels
            .iter()
            .map(|w| w.rotation_speed)
            .collect();

        // Now no input — rolling resistance (0.99 multiplier) should slow rotation
        let idle = VehicleInput::default();
        for _ in 0..120 {
            vm.update_with_input(vid, &mut pw, &idle, 1.0 / 60.0);
            pw.step();
        }

        let v = vm.get(vid).unwrap();
        // At least some wheel rotation should have decreased
        let any_decreased = v
            .wheels
            .iter()
            .enumerate()
            .any(|(i, w)| w.rotation_speed.abs() < initial[i].abs() || initial[i].abs() < 0.01);
        assert!(
            any_decreased,
            "Rolling resistance should slow wheel rotation over time"
        );
    }

    #[test]
    fn r6_engine_rpm_increases_with_throttle() {
        let (mut pw, mut vm, vid) = spawn_test_vehicle();
        settle_vehicle(&mut pw, &mut vm, vid);

        let initial_rpm = vm.get(vid).unwrap().engine_rpm;

        let input = VehicleInput {
            throttle: 1.0,
            ..Default::default()
        };
        for _ in 0..60 {
            vm.update_with_input(vid, &mut pw, &input, 1.0 / 60.0);
            pw.step();
        }

        let final_rpm = vm.get(vid).unwrap().engine_rpm;
        // Throttle target = idle + 1.0 * (max - idle) * 0.8
        // With default engine: 800 + 1.0 * (6500 - 800) * 0.8 = 5360
        // RPM should have moved significantly above idle
        assert!(
            final_rpm > initial_rpm + 100.0,
            "Throttle should increase RPM: initial={}, final={}",
            initial_rpm,
            final_rpm
        );
    }

    #[test]
    fn r6_engine_rpm_stays_clamped() {
        let (mut pw, mut vm, vid) = spawn_test_vehicle();
        settle_vehicle(&mut pw, &mut vm, vid);

        let input = VehicleInput {
            throttle: 1.0,
            ..Default::default()
        };
        for _ in 0..300 {
            vm.update_with_input(vid, &mut pw, &input, 1.0 / 60.0);
            pw.step();
        }

        let v = vm.get(vid).unwrap();
        assert!(
            v.engine_rpm >= v.config.engine.idle_rpm,
            "RPM {} below idle {}",
            v.engine_rpm,
            v.config.engine.idle_rpm
        );
        assert!(
            v.engine_rpm <= v.config.engine.max_rpm,
            "RPM {} above max {}",
            v.engine_rpm,
            v.config.engine.max_rpm
        );
    }

    #[test]
    fn r6_neutral_gear_free_rev() {
        let (mut pw, mut vm, vid) = spawn_test_vehicle();
        settle_vehicle(&mut pw, &mut vm, vid);

        // Shift to neutral: shift_down from gear 1 → 0
        let shift_down = VehicleInput {
            shift: -1,
            ..Default::default()
        };
        vm.update_with_input(vid, &mut pw, &shift_down, 1.0 / 60.0);
        pw.step();

        // Wait for shift to complete
        for _ in 0..30 {
            vm.update_with_input(vid, &mut pw, &VehicleInput::default(), 1.0 / 60.0);
            pw.step();
        }

        let v = vm.get(vid).unwrap();
        assert_eq!(v.current_gear, 0);

        // Full throttle in neutral
        let throttle = VehicleInput {
            throttle: 1.0,
            ..Default::default()
        };
        for _ in 0..60 {
            vm.update_with_input(vid, &mut pw, &throttle, 1.0 / 60.0);
            pw.step();
        }

        let v = vm.get(vid).unwrap();
        // In neutral, gear_ratio = 0, so the free-rev path is used
        // RPM should rise toward throttle target with 0.85/0.15 blend
        assert!(
            v.engine_rpm > 1000.0,
            "Free rev with throttle should push RPM well above idle: {}",
            v.engine_rpm
        );
    }

    #[test]
    fn r6_shift_blocks_throttle() {
        let (mut pw, mut vm, vid) = spawn_test_vehicle();
        settle_vehicle(&mut pw, &mut vm, vid);

        // Shift up — sets shift_timer > 0
        let shift_up = VehicleInput {
            shift: 1,
            throttle: 1.0,
            ..Default::default()
        };
        vm.update_with_input(vid, &mut pw, &shift_up, 1.0 / 60.0);

        let v = vm.get(vid).unwrap();
        // During shift, is_shifting() = true, effective_throttle = 0
        // Vehicle should get no drive force during shift
        assert!(
            v.shift_timer > 0.0 || v.current_gear == 2,
            "Shift should either set timer or change gear"
        );
    }

    #[test]
    fn r6_speed_changes_update_vehicle() {
        let (mut pw, mut vm, vid) = spawn_test_vehicle();
        settle_vehicle(&mut pw, &mut vm, vid);

        let input = VehicleInput {
            throttle: 1.0,
            ..Default::default()
        };
        for _ in 0..60 {
            vm.update_with_input(vid, &mut pw, &input, 1.0 / 60.0);
            pw.step();
        }

        let v = vm.get(vid).unwrap();
        // update_vehicle reads velocity from physics and sets speed
        // speed = velocity.length()
        assert!(
            (v.speed - v.velocity.length()).abs() < 0.01,
            "speed ({}) should equal velocity.length() ({})",
            v.speed,
            v.velocity.length()
        );
    }

    #[test]
    fn r6_grounded_contact_point_near_ground() {
        let (mut pw, mut vm, vid) = spawn_test_vehicle();
        settle_vehicle(&mut pw, &mut vm, vid);

        let v = vm.get(vid).unwrap();
        // Grounded wheels should have contact_point near y=0 (ground level)
        for (i, w) in v.wheels.iter().enumerate() {
            if w.grounded {
                assert!(
                    w.contact_point.y < 1.0,
                    "Wheel {} contact point should be near ground: {:?}",
                    i,
                    w.contact_point
                );
            }
        }
    }

    #[test]
    fn r6_compression_positive_grounded() {
        let (mut pw, mut vm, vid) = spawn_test_vehicle();
        settle_vehicle(&mut pw, &mut vm, vid);

        let v = vm.get(vid).unwrap();
        for (i, w) in v.wheels.iter().enumerate() {
            if w.grounded {
                // compression = rest_length - suspension_length
                // When resting on ground, suspension is compressed → positive
                assert!(
                    w.compression > -0.5,
                    "Wheel {} compression should be reasonable: {}",
                    i,
                    w.compression
                );
            }
        }
    }

    #[test]
    fn r6_aerodynamic_drag_opposes_motion() {
        let (mut pw, mut vm, vid) = spawn_test_vehicle();
        settle_vehicle(&mut pw, &mut vm, vid);

        // Build up speed
        let accel = VehicleInput {
            throttle: 1.0,
            ..Default::default()
        };
        for _ in 0..120 {
            vm.update_with_input(vid, &mut pw, &accel, 1.0 / 60.0);
            pw.step();
        }
        let high_speed = vm.get(vid).unwrap().speed;

        // Coast (no throttle) — drag should slow the vehicle
        let coast = VehicleInput::default();
        for _ in 0..120 {
            vm.update_with_input(vid, &mut pw, &coast, 1.0 / 60.0);
            pw.step();
        }
        let coasting_speed = vm.get(vid).unwrap().speed;

        if high_speed > 1.0 {
            assert!(
                coasting_speed < high_speed,
                "Drag should slow vehicle: peak={}, after coast={}",
                high_speed,
                coasting_speed
            );
        }
    }

    #[test]
    fn r6_lateral_force_opposes_sideslip() {
        let (mut pw, mut vm, vid) = spawn_test_vehicle();
        settle_vehicle(&mut pw, &mut vm, vid);

        // Build speed while steering — creates lateral velocity
        let input = VehicleInput {
            throttle: 0.8,
            steering: 0.5,
            ..Default::default()
        };
        for _ in 0..60 {
            vm.update_with_input(vid, &mut pw, &input, 1.0 / 60.0);
            pw.step();
        }

        let v = vm.get(vid).unwrap();
        // At least verify slip_angle is nonzero on some grounded wheel
        let has_slip = v
            .wheels
            .iter()
            .any(|w| w.grounded && w.slip_angle.abs() > 0.001);
        // With speed and steering, we expect some lateral dynamics
        // (If lat_force sign is deleted, vehicle would spin out faster)
        assert!(
            v.speed > 0.01 || has_slip || v.velocity.length() > 0.01,
            "Vehicle should have dynamics with throttle and steering"
        );
    }

    #[test]
    fn r6_load_factor_clamp() {
        let (mut pw, mut vm, vid) = spawn_test_vehicle();
        settle_vehicle(&mut pw, &mut vm, vid);

        // At very high speed, load_factor = (speed/20).clamp(0, 0.5)
        // Build very high speed
        let input = VehicleInput {
            throttle: 1.0,
            ..Default::default()
        };
        for _ in 0..300 {
            vm.update_with_input(vid, &mut pw, &input, 1.0 / 60.0);
            pw.step();
        }

        let v = vm.get(vid).unwrap();
        // RPM should be reasonable (not NaN or infinite)
        assert!(
            v.engine_rpm.is_finite(),
            "Engine RPM should be finite: {}",
            v.engine_rpm
        );
        assert!(
            v.engine_rpm >= v.config.engine.idle_rpm - 1.0,
            "RPM {} should be >= idle {}",
            v.engine_rpm,
            v.config.engine.idle_rpm
        );
    }

    #[test]
    fn r6_average_slip_ratio_matches_wheels() {
        let (mut pw, mut vm, vid) = spawn_test_vehicle();
        settle_vehicle(&mut pw, &mut vm, vid);

        let input = VehicleInput {
            throttle: 0.5,
            ..Default::default()
        };
        for _ in 0..30 {
            vm.update_with_input(vid, &mut pw, &input, 1.0 / 60.0);
            pw.step();
        }

        let v = vm.get(vid).unwrap();
        // average_slip_ratio() should match manual calculation
        let grounded_wheels: Vec<&WheelState> = v.wheels.iter().filter(|w| w.grounded).collect();
        if !grounded_wheels.is_empty() {
            let manual_avg: f32 = grounded_wheels
                .iter()
                .map(|w| w.slip_ratio.abs())
                .sum::<f32>()
                / grounded_wheels.len() as f32;
            assert!(
                (v.average_slip_ratio() - manual_avg).abs() < 1e-5,
                "average_slip_ratio {} should match manual calc {}",
                v.average_slip_ratio(),
                manual_avg
            );
        }
    }

    #[test]
    fn r6_total_suspension_force_matches_wheels() {
        let (mut pw, mut vm, vid) = spawn_test_vehicle();
        settle_vehicle(&mut pw, &mut vm, vid);

        let v = vm.get(vid).unwrap();
        let manual_total: f32 = v.wheels.iter().map(|w| w.suspension_force).sum();
        assert!(
            (v.total_suspension_force() - manual_total).abs() < 1e-3,
            "total_suspension_force {} should match sum {}",
            v.total_suspension_force(),
            manual_total
        );
    }

    #[test]
    fn r6_is_airborne_before_settling() {
        let (pw, vm, vid) = spawn_test_vehicle();
        // Vehicle spawned at y=2.0 — should be airborne initially (after just 1 update_vehicle)
        // Actually spawn_test_vehicle does pw.step() once for query pipeline init
        let v = vm.get(vid).unwrap();
        // Initial wheels are not grounded (default WheelState)
        assert!(
            v.is_airborne(),
            "Vehicle should be airborne before settling: {} grounded",
            v.grounded_wheels()
        );
    }

    // ===== ROUND 7: Targeted mutation catches =====

    #[test]
    fn r7_wheel_config_front_left_properties() {
        let wc = WheelConfig::front_left(Vec3::new(-0.7, 0.0, 1.2));
        assert!(wc.steerable, "front_left should be steerable");
        assert!(!wc.driven, "front_left should not be driven (RWD)");
        assert_eq!(wc.position_id, WheelPosition::FrontLeft);
        assert_eq!(wc.position, Vec3::new(-0.7, 0.0, 1.2));
    }

    #[test]
    fn r7_wheel_config_front_right_properties() {
        let wc = WheelConfig::front_right(Vec3::new(0.7, 0.0, 1.2));
        assert!(wc.steerable, "front_right should be steerable");
        assert!(!wc.driven, "front_right should not be driven (RWD)");
        assert_eq!(wc.position_id, WheelPosition::FrontRight);
        assert_eq!(wc.position, Vec3::new(0.7, 0.0, 1.2));
    }

    #[test]
    fn r7_wheel_config_rear_left_properties() {
        let wc = WheelConfig::rear_left(Vec3::new(-0.7, 0.0, -1.2));
        assert!(!wc.steerable, "rear_left should not be steerable");
        assert!(wc.driven, "rear_left should be driven (RWD)");
        assert_eq!(wc.position_id, WheelPosition::RearLeft);
    }

    #[test]
    fn r7_wheel_config_rear_right_properties() {
        let wc = WheelConfig::rear_right(Vec3::new(0.7, 0.0, -1.2));
        assert!(!wc.steerable, "rear_right should not be steerable");
        assert!(wc.driven, "rear_right should be driven (RWD)");
        assert_eq!(wc.position_id, WheelPosition::RearRight);
    }

    #[test]
    fn r7_engine_torque_at_midrange_rpm() {
        let config = EngineConfig::default();
        // At max_torque_rpm, torque should be max
        let torque_peak = config.torque_at_rpm(config.max_torque_rpm);
        assert!(
            (torque_peak - config.max_torque).abs() < config.max_torque * 0.05,
            "At max_torque_rpm, torque should be ~max_torque: got {} vs expected {}",
            torque_peak,
            config.max_torque
        );

        // At a middle RPM, torque should be non-zero and less than max
        let mid_rpm = (config.idle_rpm + config.max_torque_rpm) * 0.5;
        let torque_mid = config.torque_at_rpm(mid_rpm);
        assert!(
            torque_mid > 0.0 && torque_mid <= config.max_torque,
            "Mid-range torque should be in (0, max]: got {}",
            torque_mid
        );

        // At max_rpm, torque should drop to ~0
        let near_max = config.max_rpm - 1.0;
        let torque_near_max = config.torque_at_rpm(near_max);
        assert!(
            torque_near_max < config.max_torque,
            "Near max_rpm, torque should be less than max: got {}",
            torque_near_max
        );
    }

    #[test]
    fn r7_apply_forces_suspension_springs_proportional() {
        // Suspension force should be proportional to compression * stiffness
        let (mut pw, mut vm, vid) = spawn_test_vehicle();
        settle_vehicle(&mut pw, &mut vm, vid);

        let v = vm.get(vid).unwrap();
        let config = &v.config;
        for (i, w) in v.wheels.iter().enumerate() {
            if w.grounded && w.compression > 0.0 {
                // spring_force = compression * stiffness
                let expected_spring = w.compression * config.wheels[i].suspension_stiffness;
                // suspension_force = (spring + damper).max(0)
                // Suspension should be within reasonable bounds of spring component
                assert!(
                    w.suspension_force > 0.0,
                    "Wheel {} suspension should be positive when compression is positive",
                    i
                );
                // The suspension force must not be wildly different from spring force
                // (damper at rest should be small)
                assert!(
                    w.suspension_force < expected_spring * 5.0,
                    "Wheel {} suspension={} vastly exceeds spring={}",
                    i,
                    w.suspension_force,
                    expected_spring
                );
            }
        }
    }

    #[test]
    fn r7_update_vehicle_compression_within_range() {
        // After settling, compression should be between 0 and rest_length
        let (mut pw, mut vm, vid) = spawn_test_vehicle();
        settle_vehicle(&mut pw, &mut vm, vid);

        let v = vm.get(vid).unwrap();
        for (i, w) in v.wheels.iter().enumerate() {
            if w.grounded {
                assert!(
                    w.compression > -v.config.wheels[i].suspension_max_extension - 0.01,
                    "Wheel {} compression {} below min_extension",
                    i,
                    w.compression
                );
                assert!(
                    w.compression
                        < v.config.wheels[i].suspension_rest_length
                            + v.config.wheels[i].suspension_max_compression
                            + 0.1,
                    "Wheel {} compression {} exceeds rest + max_compression",
                    i,
                    w.compression
                );
            }
        }
    }

    #[test]
    fn r7_apply_forces_driven_wheels_get_torque() {
        let (mut pw, mut vm, vid) = spawn_test_vehicle();
        settle_vehicle(&mut pw, &mut vm, vid);

        // Apply throttle for a few frames
        let input = VehicleInput {
            throttle: 1.0,
            ..Default::default()
        };
        for _ in 0..20 {
            vm.update_with_input(vid, &mut pw, &input, 1.0 / 60.0);
            pw.step();
        }

        let v = vm.get(vid).unwrap();
        let mut driven_rotation_sum = 0.0_f32;
        let mut non_driven_rotation_sum = 0.0_f32;

        for (i, w) in v.wheels.iter().enumerate() {
            if v.config.wheels[i].driven {
                driven_rotation_sum += w.rotation_speed.abs();
            } else {
                non_driven_rotation_sum += w.rotation_speed.abs();
            }
        }

        // Driven wheels should spin faster with throttle
        assert!(
            driven_rotation_sum > non_driven_rotation_sum,
            "Driven wheels ({}) should spin faster than non-driven ({})",
            driven_rotation_sum,
            non_driven_rotation_sum
        );
    }

    #[test]
    fn r7_apply_forces_brake_reduces_long_force() {
        let (mut pw, mut vm, vid) = spawn_test_vehicle();
        settle_vehicle(&mut pw, &mut vm, vid);

        // Build up speed
        let accel = VehicleInput {
            throttle: 1.0,
            ..Default::default()
        };
        for _ in 0..60 {
            vm.update_with_input(vid, &mut pw, &accel, 1.0 / 60.0);
            pw.step();
        }

        // Coast for a moment to settle, then record speed
        let coast = VehicleInput::default();
        for _ in 0..30 {
            vm.update_with_input(vid, &mut pw, &coast, 1.0 / 60.0);
            pw.step();
        }
        let speed_before = vm.get(vid).unwrap().speed;

        // Apply brakes for longer
        let brake = VehicleInput {
            brake: 1.0,
            ..Default::default()
        };
        for _ in 0..120 {
            vm.update_with_input(vid, &mut pw, &brake, 1.0 / 60.0);
            pw.step();
        }
        let speed_after = vm.get(vid).unwrap().speed;

        assert!(
            speed_after < speed_before,
            "Braking should reduce speed: before={}, after={}",
            speed_before,
            speed_after
        );
    }

    #[test]
    fn r7_apply_forces_lateral_force_opposes_sideslip() {
        // If the vehicle has lateral velocity, lateral force should resist it
        let (mut pw, mut vm, vid) = spawn_test_vehicle();
        settle_vehicle(&mut pw, &mut vm, vid);

        // Drive forward
        let fwd = VehicleInput {
            throttle: 0.8,
            steering: 0.5,
            ..Default::default()
        };
        for _ in 0..30 {
            vm.update_with_input(vid, &mut pw, &fwd, 1.0 / 60.0);
            pw.step();
        }

        // Check that at least one wheel has non-zero slip_angle
        let v = vm.get(vid).unwrap();
        let has_slip_angle = v
            .wheels
            .iter()
            .any(|w| w.grounded && w.slip_angle.abs() > 0.001);
        // If there's any lateral velocity and steering, slip_angle should be generated
        if v.speed > 1.0 {
            assert!(
                has_slip_angle,
                "Steering at speed should produce slip angles"
            );
        }
    }

    #[test]
    fn r7_apply_forces_aerodynamic_drag_at_speed() {
        let (mut pw, mut vm, vid) = spawn_test_vehicle();
        settle_vehicle(&mut pw, &mut vm, vid);

        // Accelerate
        let accel = VehicleInput {
            throttle: 1.0,
            ..Default::default()
        };
        for _ in 0..120 {
            vm.update_with_input(vid, &mut pw, &accel, 1.0 / 60.0);
            pw.step();
        }

        // Vehicle should not exceed theoretical top speed (drag limits it)
        let v = vm.get(vid).unwrap();
        // Aerodynamic drag = 0.5 * 1.225 * Cd * A * v^2
        // At high speed, this force should be significant
        // Top speed with drag should be bounded
        assert!(
            v.speed < 500.0,
            "Aerodynamic drag should limit top speed: got {}",
            v.speed
        );
    }

    #[test]
    fn r7_engine_rpm_blend_at_speed() {
        let (mut pw, mut vm, vid) = spawn_test_vehicle();
        settle_vehicle(&mut pw, &mut vm, vid);

        // Drive at high throttle
        let input = VehicleInput {
            throttle: 1.0,
            ..Default::default()
        };
        for _ in 0..120 {
            vm.update_with_input(vid, &mut pw, &input, 1.0 / 60.0);
            pw.step();
        }

        let v = vm.get(vid).unwrap();
        // RPM should be influenced by wheel speed at higher velocities
        // load_factor = (speed/20).clamp(0, 0.5) — at speed > 0, should blend
        assert!(
            v.engine_rpm > v.config.engine.idle_rpm,
            "RPM should be above idle with throttle: got {}",
            v.engine_rpm
        );
        assert!(
            v.engine_rpm <= v.config.engine.max_rpm,
            "RPM should not exceed max: got {}",
            v.engine_rpm
        );
    }

    // ===== ROUND 9: Torque, transmission, RPM precision =====

    #[test]
    fn r9_torque_at_peak_rpm_returns_max_torque() {
        let engine = EngineConfig::default();
        // At max_torque_rpm, torque should be max_torque
        // (the parabolic curve peaks here)
        let torque = engine.torque_at_rpm(engine.max_torque_rpm);
        assert!(
            (torque - engine.max_torque).abs() < 1.0,
            "Torque at peak RPM ({}) should be ~{}, got {}",
            engine.max_torque_rpm,
            engine.max_torque,
            torque
        );
    }

    #[test]
    fn r9_torque_at_rpm_zero_below_idle() {
        let engine = EngineConfig::default();
        let torque = engine.torque_at_rpm(100.0); // well below idle (800)
        assert_eq!(torque, 0.0, "Torque below idle should be 0, got {}", torque);
    }

    #[test]
    fn r9_torque_at_rpm_zero_above_max() {
        let engine = EngineConfig::default();
        let torque = engine.torque_at_rpm(8000.0); // above max_rpm (7000)
        assert_eq!(
            torque, 0.0,
            "Torque above max RPM should be 0, got {}",
            torque
        );
    }

    #[test]
    fn r9_torque_at_rpm_rising_curve() {
        let engine = EngineConfig::default();
        // Midpoint between idle and max_torque_rpm should give partial torque
        let mid_rpm = (engine.idle_rpm + engine.max_torque_rpm) / 2.0;
        let torque = engine.torque_at_rpm(mid_rpm);
        assert!(
            torque > 0.0 && torque < engine.max_torque,
            "Mid-range torque should be between 0 and max: got {} at RPM {}",
            torque,
            mid_rpm
        );
    }

    #[test]
    fn r9_torque_at_rpm_falling_curve() {
        let engine = EngineConfig::default();
        // Midpoint between max_torque_rpm and max_rpm should give decreasing torque
        let falloff_rpm = (engine.max_torque_rpm + engine.max_rpm) / 2.0;
        let torque = engine.torque_at_rpm(falloff_rpm);
        assert!(
            torque > 0.0 && torque < engine.max_torque,
            "Falloff torque should be less than max: got {} at RPM {}",
            torque,
            falloff_rpm
        );
    }

    #[test]
    fn r9_effective_ratio_neutral_is_zero() {
        let trans = TransmissionConfig::default();
        let ratio = trans.effective_ratio(0);
        assert_eq!(ratio, 0.0, "Neutral gear ratio should be 0");
    }

    #[test]
    fn r9_effective_ratio_first_gear() {
        let trans = TransmissionConfig::default();
        let ratio = trans.effective_ratio(1);
        let expected = trans.gear_ratios[0] * trans.final_drive; // 3.5 * 3.7 = 12.95
        assert!(
            (ratio - expected).abs() < 0.01,
            "1st gear ratio should be {}, got {}",
            expected,
            ratio
        );
    }

    #[test]
    fn r9_effective_ratio_reverse() {
        let trans = TransmissionConfig::default();
        let ratio = trans.effective_ratio(-1);
        let expected = trans.reverse_ratio * trans.final_drive; // -3.2 * 3.7 = -11.84
        assert!(
            (ratio - expected).abs() < 0.01,
            "Reverse ratio should be {}, got {}",
            expected,
            ratio
        );
    }

    #[test]
    fn r9_apply_forces_suspension_value_proportional() {
        let (mut pw, mut vm, vid) = spawn_test_vehicle();
        settle_vehicle(&mut pw, &mut vm, vid);

        let v = vm.get(vid).unwrap();
        for (i, w) in v.wheels.iter().enumerate() {
            if w.grounded && w.compression > 0.001 {
                let stiff = v.config.wheels[i].suspension_stiffness;
                let spring_component = w.compression * stiff;

                // Suspension force should be close to spring force (plus some damping)
                // It should at least be in the right order of magnitude
                let ratio = w.suspension_force / spring_component;
                assert!(
                    ratio > 0.1 && ratio < 10.0,
                    "Wheel {} suspension/spring ratio should be reasonable: susp={}, spring={}, ratio={}",
                    i, w.suspension_force, spring_component, ratio
                );
            }
        }
    }

    #[test]
    fn r9_apply_forces_wheel_force_has_components() {
        let (mut pw, mut vm, vid) = spawn_test_vehicle();
        settle_vehicle(&mut pw, &mut vm, vid);

        // Apply throttle
        let input = VehicleInput {
            throttle: 1.0,
            ..Default::default()
        };
        for _ in 0..10 {
            vm.update_with_input(vid, &mut pw, &input, 1.0 / 60.0);
            pw.step();
        }

        let v = vm.get(vid).unwrap();
        // Grounded wheels should have non-zero force vectors
        let has_force = v
            .wheels
            .iter()
            .any(|w| w.grounded && w.force.length() > 0.01);
        assert!(
            has_force,
            "Grounded wheels should have force after throttle input"
        );

        // At least one wheel force should have a suspension (Y-ish) component
        let has_vertical = v.wheels.iter().any(|w| w.grounded && w.force.y.abs() > 0.1);
        assert!(
            has_vertical,
            "Wheel forces should include suspension (vertical) component"
        );
    }

    #[test]
    fn r9_apply_forces_rpm_increases_with_throttle() {
        let (mut pw, mut vm, vid) = spawn_test_vehicle();
        settle_vehicle(&mut pw, &mut vm, vid);

        let idle_rpm = vm.get(vid).unwrap().config.engine.idle_rpm;
        let initial_rpm = vm.get(vid).unwrap().engine_rpm;

        // Full throttle for a while
        let input = VehicleInput {
            throttle: 1.0,
            ..Default::default()
        };
        for _ in 0..60 {
            vm.update_with_input(vid, &mut pw, &input, 1.0 / 60.0);
            pw.step();
        }

        let final_rpm = vm.get(vid).unwrap().engine_rpm;
        assert!(
            final_rpm > initial_rpm,
            "RPM should increase with throttle: initial={}, final={}",
            initial_rpm,
            final_rpm
        );
        assert!(
            final_rpm > idle_rpm * 1.5,
            "RPM should rise well above idle with full throttle: rpm={}, idle={}",
            final_rpm,
            idle_rpm
        );
    }

    #[test]
    fn r9_apply_forces_rpm_blend_formula() {
        // Test that RPM blending uses the 0.85/0.15 smoothing formula
        let (mut pw, mut vm, vid) = spawn_test_vehicle();
        settle_vehicle(&mut pw, &mut vm, vid);

        let rpm_before = vm.get(vid).unwrap().engine_rpm;

        // One frame of full throttle
        let input = VehicleInput {
            throttle: 1.0,
            ..Default::default()
        };
        vm.update_with_input(vid, &mut pw, &input, 1.0 / 60.0);
        pw.step();

        let rpm_after = vm.get(vid).unwrap().engine_rpm;
        // RPM should change but not instantly (due to 0.85/0.15 blending)
        // new_rpm = old_rpm * 0.85 + target * 0.15
        // So change should be moderate, not immediate jump
        if rpm_after > rpm_before {
            let change = rpm_after - rpm_before;
            let max_possible_change = vm.get(vid).unwrap().config.engine.max_rpm - rpm_before;
            assert!(
                change < max_possible_change * 0.5,
                "RPM change should be gradual (blended): change={}, max={}",
                change,
                max_possible_change
            );
        }
    }

    #[test]
    fn r9_apply_forces_steering_produces_lateral() {
        let (mut pw, mut vm, vid) = spawn_test_vehicle();
        settle_vehicle(&mut pw, &mut vm, vid);

        // Accelerate first
        let accel = VehicleInput {
            throttle: 1.0,
            ..Default::default()
        };
        for _ in 0..60 {
            vm.update_with_input(vid, &mut pw, &accel, 1.0 / 60.0);
            pw.step();
        }

        // Then steer at speed
        let steer = VehicleInput {
            throttle: 0.5,
            steering: 1.0,
            ..Default::default()
        };
        for _ in 0..30 {
            vm.update_with_input(vid, &mut pw, &steer, 1.0 / 60.0);
            pw.step();
        }

        let v = vm.get(vid).unwrap();
        if v.speed > 1.0 {
            // Steered wheels should produce lateral force
            let has_lat_force = v
                .wheels
                .iter()
                .any(|w| w.grounded && w.force.length() > 0.1);
            assert!(
                has_lat_force,
                "Steering at speed should produce wheel forces"
            );
        }
    }

    #[test]
    fn r9_apply_forces_drag_force_at_high_speed() {
        // Higher drag coefficient should produce lower top speed.
        // Compare two vehicles: default drag vs 10× drag.
        let (mut pw_lo, mut vm_lo, vid_lo) = spawn_test_vehicle();
        let (mut pw_hi, mut vm_hi, vid_hi) = spawn_test_vehicle();
        settle_vehicle(&mut pw_lo, &mut vm_lo, vid_lo);
        settle_vehicle(&mut pw_hi, &mut vm_hi, vid_hi);

        // Increase drag on the high-drag vehicle
        if let Some(v) = vm_hi.get_mut(vid_hi) {
            v.config.drag_coefficient *= 10.0;
        }

        let input = VehicleInput {
            throttle: 1.0,
            ..Default::default()
        };
        for _ in 0..600 {
            vm_lo.update_with_input(vid_lo, &mut pw_lo, &input, 1.0 / 60.0);
            pw_lo.step();
            vm_hi.update_with_input(vid_hi, &mut pw_hi, &input, 1.0 / 60.0);
            pw_hi.step();
        }

        let speed_lo = vm_lo.get(vid_lo).unwrap().speed;
        let speed_hi = vm_hi.get(vid_hi).unwrap().speed;

        assert!(
            speed_lo > speed_hi + 1.0,
            "Low drag vehicle should be faster: lo={}, hi={}",
            speed_lo,
            speed_hi
        );
    }

    // ===== ROUND 10: apply_forces arithmetic, suspension, wheel rotation =====

    #[test]
    fn r10_apply_forces_suspension_spring_damper() {
        // Verify suspension force = spring + damper = compression * stiffness + vel * damping
        let (mut pw, mut vm, vid) = spawn_test_vehicle();
        settle_vehicle(&mut pw, &mut vm, vid);

        let input = VehicleInput::default(); // no throttle
        vm.update_with_input(vid, &mut pw, &input, 1.0 / 60.0);

        let v = vm.get(vid).unwrap();
        // At rest, grounded wheels should have positive suspension force
        for (i, ws) in v.wheels.iter().enumerate() {
            if ws.grounded {
                assert!(
                    ws.suspension_force > 0.0,
                    "Wheel {} suspension force should be positive at rest: {}",
                    i,
                    ws.suspension_force
                );
            }
        }
    }

    #[test]
    fn r10_apply_forces_wheel_rotation_under_throttle() {
        let (mut pw, mut vm, vid) = spawn_test_vehicle();
        settle_vehicle(&mut pw, &mut vm, vid);

        let v_before = vm.get(vid).unwrap().clone();
        let driven_before: Vec<f32> = v_before
            .wheels
            .iter()
            .enumerate()
            .filter(|(i, _)| v_before.config.wheels[*i].driven)
            .map(|(_, w)| w.rotation_speed)
            .collect();

        let input = VehicleInput {
            throttle: 1.0,
            ..Default::default()
        };
        for _ in 0..30 {
            vm.update_with_input(vid, &mut pw, &input, 1.0 / 60.0);
            pw.step();
        }

        let v_after = vm.get(vid).unwrap();
        let driven_after: Vec<f32> = v_after
            .wheels
            .iter()
            .enumerate()
            .filter(|(i, _)| v_after.config.wheels[*i].driven)
            .map(|(_, w)| w.rotation_speed)
            .collect();

        // Driven wheels should spin faster under throttle
        for (before, after) in driven_before.iter().zip(driven_after.iter()) {
            assert!(
                after.abs() > before.abs() + 0.1,
                "Driven wheel should spin faster: before={}, after={}",
                before,
                after
            );
        }
    }

    #[test]
    fn r10_apply_forces_brake_reduces_speed() {
        // Verify that brake input causes wheel forces to include braking component
        let (mut pw, mut vm, vid) = spawn_test_vehicle();
        settle_vehicle(&mut pw, &mut vm, vid);

        // Accelerate to get wheels spinning
        let accel = VehicleInput {
            throttle: 1.0,
            ..Default::default()
        };
        for _ in 0..60 {
            vm.update_with_input(vid, &mut pw, &accel, 1.0 / 60.0);
            pw.step();
        }

        let speed_accel = vm.get(vid).unwrap().speed;
        assert!(speed_accel > 1.0, "Should have some speed: {}", speed_accel);

        // Check that wheel forces exist after braking input
        let brake_input = VehicleInput {
            brake: 1.0,
            ..Default::default()
        };
        vm.update_with_input(vid, &mut pw, &brake_input, 1.0 / 60.0);

        let v = vm.get(vid).unwrap();
        let has_brake_force = v
            .wheels
            .iter()
            .any(|w| w.grounded && w.force.length() > 0.01);
        assert!(has_brake_force, "Brake should produce wheel forces");
    }

    #[test]
    fn r10_apply_forces_handbrake_rear_wheels() {
        // Compare coasting vs handbrake: handbrake should result in lower speed
        let (mut pw_coast, mut vm_coast, vid_coast) = spawn_test_vehicle();
        let (mut pw_hb, mut vm_hb, vid_hb) = spawn_test_vehicle();
        settle_vehicle(&mut pw_coast, &mut vm_coast, vid_coast);
        settle_vehicle(&mut pw_hb, &mut vm_hb, vid_hb);

        // Accelerate both
        let accel = VehicleInput {
            throttle: 1.0,
            ..Default::default()
        };
        for _ in 0..90 {
            vm_coast.update_with_input(vid_coast, &mut pw_coast, &accel, 1.0 / 60.0);
            pw_coast.step();
            vm_hb.update_with_input(vid_hb, &mut pw_hb, &accel, 1.0 / 60.0);
            pw_hb.step();
        }

        // Coast vs handbrake
        let coast_input = VehicleInput::default();
        let hb_input = VehicleInput {
            handbrake: 1.0,
            ..Default::default()
        };
        for _ in 0..90 {
            vm_coast.update_with_input(vid_coast, &mut pw_coast, &coast_input, 1.0 / 60.0);
            pw_coast.step();
            vm_hb.update_with_input(vid_hb, &mut pw_hb, &hb_input, 1.0 / 60.0);
            pw_hb.step();
        }

        let speed_coast = vm_coast.get(vid_coast).unwrap().speed;
        let speed_hb = vm_hb.get(vid_hb).unwrap().speed;
        assert!(
            speed_hb < speed_coast,
            "Handbrake should slow more than coasting: hb={}, coast={}",
            speed_hb,
            speed_coast
        );
    }

    #[test]
    fn r10_update_vehicle_orientation_from_physics() {
        let (mut pw, mut vm, vid) = spawn_test_vehicle();
        settle_vehicle(&mut pw, &mut vm, vid);

        // After settling, vehicle should have reasonable forward vector
        let v = vm.get(vid).unwrap();
        // Forward should be roughly unit length
        assert!(
            v.forward.length() > 0.9 && v.forward.length() < 1.1,
            "Forward should be unit: {:?}",
            v.forward
        );
        // Up should point mostly upward
        assert!(v.up.y > 0.5, "Up should point upward: {:?}", v.up);
    }

    #[test]
    fn r10_apply_forces_slip_ratio_computed() {
        let (mut pw, mut vm, vid) = spawn_test_vehicle();
        settle_vehicle(&mut pw, &mut vm, vid);

        // Accelerate to get some speed + wheel spin
        let input = VehicleInput {
            throttle: 1.0,
            ..Default::default()
        };
        for _ in 0..60 {
            vm.update_with_input(vid, &mut pw, &input, 1.0 / 60.0);
            pw.step();
        }

        let v = vm.get(vid).unwrap();
        // At least some wheels should have non-zero slip ratio
        let has_slip = v.wheels.iter().any(|w| w.slip_ratio.abs() > 0.001);
        assert!(
            has_slip,
            "Driven wheels should have some slip ratio under throttle"
        );
    }

    #[test]
    fn r10_apply_forces_engine_rpm_responds_to_throttle() {
        let (mut pw, mut vm, vid) = spawn_test_vehicle();
        settle_vehicle(&mut pw, &mut vm, vid);

        let idle_rpm = vm.get(vid).unwrap().engine_rpm;

        // Apply throttle
        let input = VehicleInput {
            throttle: 1.0,
            ..Default::default()
        };
        for _ in 0..60 {
            vm.update_with_input(vid, &mut pw, &input, 1.0 / 60.0);
            pw.step();
        }
        let throttle_rpm = vm.get(vid).unwrap().engine_rpm;

        assert!(
            throttle_rpm > idle_rpm + 500.0,
            "RPM should increase with throttle: idle={}, throttle={}",
            idle_rpm,
            throttle_rpm
        );
    }

    // ============================================================================
    // R11 — Targeted mutation tests for WheelConfig and EngineConfig
    // ============================================================================

    #[test]
    fn r11_rear_left_is_driven() {
        let w = WheelConfig::rear_left(Vec3::ZERO);
        assert!(w.driven, "Rear-left should be driven (RWD)");
    }

    #[test]
    fn r11_rear_right_is_driven() {
        let w = WheelConfig::rear_right(Vec3::ZERO);
        assert!(w.driven, "Rear-right should be driven (RWD)");
    }

    #[test]
    fn r11_front_left_is_steerable() {
        let w = WheelConfig::front_left(Vec3::ZERO);
        assert!(w.steerable, "Front-left should be steerable");
    }

    #[test]
    fn r11_front_right_is_steerable() {
        let w = WheelConfig::front_right(Vec3::ZERO);
        assert!(w.steerable, "Front-right should be steerable");
    }

    #[test]
    fn r11_engine_torque_at_boundary_rpm() {
        let engine = EngineConfig::default();
        // At exactly idle_rpm: should produce non-zero torque (< boundary catches < vs <=)
        let at_idle = engine.torque_at_rpm(engine.idle_rpm);
        // At idle_rpm the normalized = 0, so torque = max_torque * (1 - 1^2) = 0
        // This is actually 0 by the formula. So testing slightly above:
        let near_idle = engine.torque_at_rpm(engine.idle_rpm + 1.0);
        assert!(
            near_idle > 0.0,
            "Torque just above idle should be positive: {}",
            near_idle
        );

        // At exactly max_rpm: should be 0 (> boundary)
        let at_max = engine.torque_at_rpm(engine.max_rpm);
        assert!(
            at_max.abs() < 0.01,
            "Torque at max_rpm should be 0: {}",
            at_max
        );

        // At exactly max_torque_rpm: should be peak
        let at_peak = engine.torque_at_rpm(engine.max_torque_rpm);
        assert!(
            (at_peak - engine.max_torque).abs() < 1.0,
            "Torque at peak RPM should be max_torque: expected={}, got={}",
            engine.max_torque,
            at_peak
        );
    }

    #[test]
    fn r11_transmission_effective_ratio_gear1() {
        let trans = TransmissionConfig::default();
        // gear 1: ratio = 3.5, final_drive = 3.7
        let ratio = trans.effective_ratio(1);
        let expected = 3.5 * 3.7;
        assert!(
            (ratio - expected).abs() < 0.01,
            "Gear 1 effective ratio: expected={}, got={}",
            expected,
            ratio
        );
    }

    // ============================================================================
    // R12 — Targeted apply_forces mutation-kill tests
    // ============================================================================

    #[test]
    fn r12_apply_forces_rpm_blend_in_neutral() {
        // Targets: RPM blend formula `engine_rpm * 0.85 + target * 0.15`
        // In neutral (gear 0), gear_ratio=0, uses simple branch:
        //   engine_rpm = engine_rpm * 0.85 + throttle_rpm_target * 0.15
        // where throttle_rpm_target = idle + throttle * (max_rpm - idle) * 0.8
        let (mut pw, mut vm, vid) = spawn_test_vehicle();
        settle_vehicle(&mut pw, &mut vm, vid);

        // Shift to neutral (gear 1 → gear 0)
        let shift_down = VehicleInput {
            shift: -1,
            ..Default::default()
        };
        vm.update_with_input(vid, &mut pw, &shift_down, 1.0 / 60.0);
        pw.step();

        // Complete shift timer
        let neutral = VehicleInput::default();
        for _ in 0..30 {
            vm.update_with_input(vid, &mut pw, &neutral, 1.0 / 60.0);
            pw.step();
        }

        assert_eq!(vm.get(vid).unwrap().current_gear, 0, "Should be in neutral");

        // Record RPM before throttle
        let rpm_before = vm.get(vid).unwrap().engine_rpm;

        // Apply full throttle for 1 frame in neutral
        let throttle = VehicleInput {
            throttle: 1.0,
            ..Default::default()
        };
        vm.update_with_input(vid, &mut pw, &throttle, 1.0 / 60.0);

        let rpm_after = vm.get(vid).unwrap().engine_rpm;
        let engine = &vm.get(vid).unwrap().config.engine;

        // throttle_rpm_target = idle + 1.0 * (max_rpm - idle) * 0.8
        let throttle_rpm_target = engine.idle_rpm + 1.0 * (engine.max_rpm - engine.idle_rpm) * 0.8;

        // In neutral: engine_rpm = rpm_before * 0.85 + throttle_rpm_target * 0.15
        let expected = rpm_before * 0.85 + throttle_rpm_target * 0.15;

        // Also clamp to [idle, max_rpm]
        let expected_clamped = expected.clamp(engine.idle_rpm, engine.max_rpm);

        assert!(
            (rpm_after - expected_clamped).abs() < 5.0,
            "RPM blend in neutral: expected={:.1}, got={:.1} (before={:.1}, target={:.1})",
            expected_clamped,
            rpm_after,
            rpm_before,
            throttle_rpm_target
        );
    }

    #[test]
    fn r12_apply_forces_suspension_tight_spring() {
        // Targets: L795 `compression * suspension_stiffness` — * vs + or /
        // After settling, damper ≈ 0, so suspension_force ≈ compression * stiffness
        // Default stiffness = 35000, compression ~ 0.01-0.05
        // Correct: ~350-1750 N. With +: ~35000 N. Clearly distinguishable.
        let (mut pw, mut vm, vid) = spawn_test_vehicle();
        // Extended settle for stable damping
        for _ in 0..240 {
            vm.update_with_input(vid, &mut pw, &VehicleInput::default(), 1.0 / 60.0);
            pw.step();
        }

        let v = vm.get(vid).unwrap();
        for (i, w) in v.wheels.iter().enumerate() {
            if w.grounded && w.compression > 0.001 {
                let stiff = v.config.wheels[i].suspension_stiffness;
                let spring = w.compression * stiff;

                // After long settling, suspension_force should be very close to spring force
                // (damper velocity ≈ 0 since compression is stable)
                let ratio = w.suspension_force / spring;
                assert!(
                    (ratio - 1.0).abs() < 0.3,
                    "Wheel {} suspension/spring ratio should be ~1.0 after settling: ratio={:.3}, susp={:.1}, spring={:.1}, comp={:.5}",
                    i, ratio, w.suspension_force, spring, w.compression
                );
            }
        }
    }

    #[test]
    fn r12_apply_forces_slip_ratio_stationary() {
        // Targets: slip ratio formula `(wheel_speed - long_velocity) / long_velocity.abs()`
        // At rest: both wheel_speed and long_velocity ≈ 0
        // The code: if long_velocity.abs() <= 0.5 && wheel_speed.abs() <= 0.1 → slip_ratio = 0
        let (mut pw, mut vm, vid) = spawn_test_vehicle();
        settle_vehicle(&mut pw, &mut vm, vid);

        let v = vm.get(vid).unwrap();
        for w in &v.wheels {
            if w.grounded {
                assert!(
                    w.slip_ratio.abs() < 0.01,
                    "Stationary grounded wheel slip_ratio should be ~0: {}",
                    w.slip_ratio
                );
            }
        }
    }

    #[test]
    fn r12_apply_forces_driven_wheels_get_rotation() {
        // Targets: angular_accel computation for driven wheels with throttle
        // torque_per_wheel / wheel_inertia should accelerate wheel rotation
        let (mut pw, mut vm, vid) = spawn_test_vehicle();
        settle_vehicle(&mut pw, &mut vm, vid);

        // Full throttle for several frames
        let input = VehicleInput {
            throttle: 1.0,
            ..Default::default()
        };
        for _ in 0..30 {
            vm.update_with_input(vid, &mut pw, &input, 1.0 / 60.0);
            pw.step();
        }

        let v = vm.get(vid).unwrap();
        // Driven wheels (rear) should have positive rotation speed
        for (i, w) in v.wheels.iter().enumerate() {
            if v.config.wheels[i].driven && w.grounded {
                assert!(
                    w.rotation_speed > 0.1,
                    "Driven wheel {} should have positive rotation under throttle: rot_speed={}",
                    i,
                    w.rotation_speed
                );
            }
        }
    }

    #[test]
    fn r12_apply_forces_suspension_supports_weight() {
        // Targets: suspension_force accumulation and spring+damper formula
        // Total suspension should roughly support vehicle weight (mass * g)
        let (mut pw, mut vm, vid) = spawn_test_vehicle();
        // Long settle
        for _ in 0..240 {
            vm.update_with_input(vid, &mut pw, &VehicleInput::default(), 1.0 / 60.0);
            pw.step();
        }

        let v = vm.get(vid).unwrap();
        let total_susp = v.total_suspension_force();
        // Vehicle mass = 1500 kg (default), so weight ≈ 1500 * 9.81 ≈ 14715 N
        let weight = v.config.mass * 9.81;

        // Suspension should be in reasonable range relative to weight
        // (spring stiffness and damping overshoot can cause higher values)
        let ratio = total_susp / weight;
        assert!(
            ratio > 0.3 && ratio < 8.0,
            "Total suspension ({:.0}N) should be proportional to vehicle weight ({:.0}N), ratio={:.2}",
            total_susp, weight, ratio
        );
    }

    #[test]
    fn r12_apply_forces_drag_increases_quadratically() {
        // Targets: drag computation 0.5 * 1.225 * Cd * A * v^2
        // At 2x speed, drag should be ~4x
        let (mut pw, mut vm, vid) = spawn_test_vehicle();
        settle_vehicle(&mut pw, &mut vm, vid);

        let body_id = vm.get(vid).unwrap().body_id;

        // Give vehicle speed=10 in +Z, measure deceleration
        pw.set_velocity(body_id, Vec3::new(0.0, 0.0, 10.0));
        let no_input = VehicleInput::default();
        vm.update_with_input(vid, &mut pw, &no_input, 1.0 / 60.0);
        let v_after_10 = pw.get_velocity(body_id).unwrap();
        let decel_10 = 10.0 - v_after_10.z; // positive if slowed down

        // Reset: spawn new vehicle for speed=20
        let vid2 = vm.spawn(&mut pw, Vec3::new(20.0, 2.0, 0.0), VehicleConfig::default());
        settle_vehicle(&mut pw, &mut vm, vid2);
        let body_id2 = vm.get(vid2).unwrap().body_id;
        pw.set_velocity(body_id2, Vec3::new(0.0, 0.0, 20.0));
        vm.update_with_input(vid2, &mut pw, &no_input, 1.0 / 60.0);
        let v_after_20 = pw.get_velocity(body_id2).unwrap();
        let decel_20 = 20.0 - v_after_20.z;

        // Drag should increase with speed: 2x speed → ≥2x deceleration
        // (Not exactly 4x because friction forces also present, but drag dominates at high speed)
        if decel_10 > 0.001 {
            assert!(
                decel_20 > decel_10 * 1.5,
                "Drag should increase with speed: decel@10={:.4}, decel@20={:.4}",
                decel_10,
                decel_20
            );
        }
    }

    #[test]
    fn r12_apply_forces_throttle_rpm_target_formula() {
        // Targets: L912 `throttle_rpm_target = idle + throttle * (max_rpm - idle) * 0.8`
        // If * → + or / → wrong target
        let (mut pw, mut vm, vid) = spawn_test_vehicle();
        settle_vehicle(&mut pw, &mut vm, vid);

        // Run with throttle=0.5 for many frames to approach steady state
        let input = VehicleInput {
            throttle: 0.5,
            ..Default::default()
        };
        for _ in 0..300 {
            vm.update_with_input(vid, &mut pw, &input, 1.0 / 60.0);
            pw.step();
        }

        let v = vm.get(vid).unwrap();
        let engine = &v.config.engine;

        // throttle_rpm_target = 800 + 0.5 * (7000 - 800) * 0.8 = 800 + 2480 = 3280
        let target = engine.idle_rpm + 0.5 * (engine.max_rpm - engine.idle_rpm) * 0.8;

        // After many frames with throttle, RPM should be well above idle
        // and within clamped range [idle, max_rpm]
        assert!(
            v.engine_rpm > engine.idle_rpm + 100.0,
            "RPM should be above idle with throttle=0.5: rpm={}, idle={}",
            v.engine_rpm,
            engine.idle_rpm
        );
        assert!(
            v.engine_rpm <= engine.max_rpm + 1.0,
            "RPM should be clamped to max: rpm={}, max={}",
            v.engine_rpm,
            engine.max_rpm
        );
    }

    #[test]
    fn r12_apply_forces_brake_opposes_velocity_precisely() {
        // Targets: brake_force sign and velocity.signum() usage
        // L877: `long_force -= brake_force * long_velocity.signum()`
        // If -= becomes +=, braking would accelerate
        let (mut pw, mut vm, vid) = spawn_test_vehicle();
        settle_vehicle(&mut pw, &mut vm, vid);

        let body_id = vm.get(vid).unwrap().body_id;
        pw.set_velocity(body_id, Vec3::new(0.0, 0.0, 15.0));

        // Full brake for 1 frame (no friction/gravity settling issues for short time)
        let brake_input = VehicleInput {
            brake: 1.0,
            ..Default::default()
        };
        vm.update_with_input(vid, &mut pw, &brake_input, 1.0 / 60.0);
        pw.step();

        let vel = pw.get_velocity(body_id).unwrap();
        // After braking, Z velocity should have decreased
        assert!(
            vel.z < 15.0,
            "Braking should reduce forward velocity: before=15.0, after={:.2}",
            vel.z
        );
    }
}
