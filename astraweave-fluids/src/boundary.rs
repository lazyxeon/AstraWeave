//! # Boundary Handling System
//!
//! Advanced boundary handling for SPH fluids using Akinci boundary particles,
//! SDF-based density contribution, and hybrid methods.
//!
//! ## Features
//!
//! - **Akinci Boundary Particles**: Accurate solid-fluid interaction (Akinci et al. 2012)
//! - **SDF-Based Density**: Fast approximate boundary density contribution
//! - **Hybrid Method**: Combines SDF speed with Akinci accuracy for friction
//! - **Slip/No-Slip Conditions**: Configurable boundary velocity constraints
//! - **Friction Model**: Surface friction with tangential damping
//!
//! ## References
//!
//! - Akinci et al. 2012: "Versatile Rigid-Fluid Coupling for Incompressible SPH"
//! - Koschier et al. 2019: "SPH Techniques for Interactive Applications" (survey)

use bytemuck::{Pod, Zeroable};

/// Minimum distance for numerical stability
const MIN_DISTANCE: f32 = 1e-6;

// =============================================================================
// BOUNDARY PARTICLE
// =============================================================================

/// Akinci boundary particle for solid surfaces
///
/// Boundary particles sample solid geometry and contribute to fluid density
/// and pressure, creating accurate solid-fluid interaction without penetration.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct BoundaryParticle {
    /// Position in world space
    pub position: [f32; 3],
    /// Boundary volume Ψ (contribution factor)
    pub volume: f32,
    /// Surface normal (pointing into fluid)
    pub normal: [f32; 3],
    /// Surface friction coefficient (0 = frictionless, 1 = high friction)
    pub friction: f32,
    /// Object ID for multiple rigid bodies
    pub object_id: u32,
    /// Boundary type (0 = static, 1 = dynamic, 2 = kinematic)
    pub boundary_type: u32,
    /// Velocity (for moving boundaries)
    pub velocity: [f32; 3],
    /// Padding for 16-byte alignment
    pub _pad: f32,
}

impl Default for BoundaryParticle {
    fn default() -> Self {
        Self {
            position: [0.0, 0.0, 0.0],
            volume: 1.0,
            normal: [0.0, 1.0, 0.0],
            friction: 0.5,
            object_id: 0,
            boundary_type: 0,
            velocity: [0.0, 0.0, 0.0],
            _pad: 0.0,
        }
    }
}

impl BoundaryParticle {
    /// Create a new boundary particle at a position with given normal
    pub fn new(position: [f32; 3], normal: [f32; 3]) -> Self {
        Self {
            position,
            normal,
            ..Default::default()
        }
    }

    /// Create with specific volume contribution
    pub fn with_volume(mut self, volume: f32) -> Self {
        self.volume = volume;
        self
    }

    /// Set friction coefficient
    pub fn with_friction(mut self, friction: f32) -> Self {
        self.friction = friction.clamp(0.0, 1.0);
        self
    }

    /// Set object ID for multi-body scenarios
    pub fn with_object_id(mut self, id: u32) -> Self {
        self.object_id = id;
        self
    }

    /// Mark as dynamic (moving) boundary
    pub fn as_dynamic(mut self, velocity: [f32; 3]) -> Self {
        self.boundary_type = 1;
        self.velocity = velocity;
        self
    }

    /// Mark as kinematic (scripted motion) boundary
    pub fn as_kinematic(mut self) -> Self {
        self.boundary_type = 2;
        self
    }
}

// =============================================================================
// BOUNDARY METHOD ENUM
// =============================================================================

/// Boundary handling method selection
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub enum BoundaryMethod {
    /// Traditional Akinci particle sampling
    /// Accurate but expensive for complex geometry
    AkinciOnly,

    /// SDF-based density contribution
    /// Fast but less accurate for friction
    SdfOnly,

    /// Hybrid approach (recommended)
    /// Uses SDF for density, Akinci for friction
    Hybrid {
        /// Use SDF for density contribution
        sdf_for_density: bool,
        /// Use boundary particles for friction
        particles_for_friction: bool,
    },
}

impl Default for BoundaryMethod {
    fn default() -> Self {
        Self::Hybrid {
            sdf_for_density: true,
            particles_for_friction: true,
        }
    }
}

// =============================================================================
// SLIP CONDITION
// =============================================================================

/// Boundary slip condition
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub enum SlipCondition {
    /// No-slip: velocity equals boundary velocity
    NoSlip,

    /// Free-slip: only normal component zeroed
    FreeSlip,

    /// Partial slip with blend factor (0 = free slip, 1 = no slip)
    PartialSlip(f32),
}

impl Default for SlipCondition {
    fn default() -> Self {
        Self::NoSlip
    }
}

impl SlipCondition {
    /// Get the slip factor (0 = free slip, 1 = no slip)
    pub fn factor(&self) -> f32 {
        match self {
            Self::NoSlip => 1.0,
            Self::FreeSlip => 0.0,
            Self::PartialSlip(f) => f.clamp(0.0, 1.0),
        }
    }
}

// =============================================================================
// BOUNDARY CONFIG
// =============================================================================

/// Configuration for boundary handling
#[derive(Clone, Debug)]
pub struct BoundaryConfig {
    /// Boundary handling method
    pub method: BoundaryMethod,
    /// Default slip condition
    pub slip_condition: SlipCondition,
    /// Default friction coefficient
    pub friction: f32,
    /// Rest density for boundary contribution
    pub rest_density: f32,
    /// Kernel support radius
    pub h: f32,
    /// Boundary particle spacing (relative to h)
    pub particle_spacing: f32,
    /// Enable adaptive sampling near fluid
    pub adaptive_sampling: bool,
    /// Restitution coefficient for collision response
    pub restitution: f32,
}

impl Default for BoundaryConfig {
    fn default() -> Self {
        Self {
            method: BoundaryMethod::default(),
            slip_condition: SlipCondition::NoSlip,
            friction: 0.5,
            rest_density: 1000.0,
            h: 0.1,
            particle_spacing: 0.5, // 0.5 * h between boundary particles
            adaptive_sampling: true,
            restitution: 0.2,
        }
    }
}

impl BoundaryConfig {
    /// Create config for static walls (typical room/container)
    pub fn static_walls() -> Self {
        Self {
            method: BoundaryMethod::SdfOnly,
            slip_condition: SlipCondition::NoSlip,
            friction: 0.8,
            ..Default::default()
        }
    }

    /// Create config for smooth surfaces (glass, polished metal)
    pub fn smooth_surface() -> Self {
        Self {
            method: BoundaryMethod::Hybrid {
                sdf_for_density: true,
                particles_for_friction: false,
            },
            slip_condition: SlipCondition::FreeSlip,
            friction: 0.1,
            restitution: 0.5,
            ..Default::default()
        }
    }

    /// Create config for rough surfaces (concrete, rock)
    pub fn rough_surface() -> Self {
        Self {
            method: BoundaryMethod::Hybrid {
                sdf_for_density: true,
                particles_for_friction: true,
            },
            slip_condition: SlipCondition::NoSlip,
            friction: 0.9,
            restitution: 0.1,
            ..Default::default()
        }
    }

    /// Create config for moving rigid bodies
    pub fn dynamic_object() -> Self {
        Self {
            method: BoundaryMethod::AkinciOnly,
            slip_condition: SlipCondition::PartialSlip(0.5),
            friction: 0.5,
            adaptive_sampling: true,
            ..Default::default()
        }
    }
}

// =============================================================================
// SPH KERNELS
// =============================================================================

/// Cubic spline kernel W(r, h)
/// Standard SPH kernel for density computation
pub fn cubic_spline_kernel(r: f32, h: f32) -> f32 {
    let q = r / h;
    let sigma = 8.0 / (std::f32::consts::PI * h.powi(3));

    if q <= 0.5 {
        sigma * (6.0 * q.powi(3) - 6.0 * q.powi(2) + 1.0)
    } else if q <= 1.0 {
        sigma * 2.0 * (1.0 - q).powi(3)
    } else {
        0.0
    }
}

/// Cubic spline kernel gradient ∇W(r, h)
pub fn cubic_spline_gradient(r: f32, h: f32) -> f32 {
    if r < MIN_DISTANCE {
        return 0.0;
    }

    let q = r / h;
    let sigma = 8.0 / (std::f32::consts::PI * h.powi(3));

    if q <= 0.5 {
        sigma * (18.0 * q.powi(2) - 12.0 * q) / h
    } else if q <= 1.0 {
        -sigma * 6.0 * (1.0 - q).powi(2) / h
    } else {
        0.0
    }
}

// =============================================================================
// DENSITY COMPUTATION
// =============================================================================

/// Compute boundary density contribution from Akinci particles
///
/// ρ_i += Σ_b ρ_0 Ψ_b W(x_i - x_b, h)
pub fn boundary_density_akinci(
    fluid_pos: [f32; 3],
    boundary_particles: &[BoundaryParticle],
    rest_density: f32,
    h: f32,
) -> f32 {
    let mut density = 0.0;

    for bp in boundary_particles {
        let dx = fluid_pos[0] - bp.position[0];
        let dy = fluid_pos[1] - bp.position[1];
        let dz = fluid_pos[2] - bp.position[2];
        let r = (dx * dx + dy * dy + dz * dz).sqrt();

        if r < h {
            density += rest_density * bp.volume * cubic_spline_kernel(r, h);
        }
    }

    density
}

/// Compute boundary density contribution from SDF
///
/// Approximates boundary density based on distance to surface.
/// Much faster than Akinci iteration for complex geometry.
pub fn boundary_density_sdf(sdf_distance: f32, rest_density: f32, h: f32) -> f32 {
    if sdf_distance <= 0.0 {
        // Inside boundary - return full density
        return rest_density;
    }

    if sdf_distance >= h {
        // Far from boundary - no contribution
        return 0.0;
    }

    // Approximate density contribution based on overlap
    let overlap = h - sdf_distance;
    let volume_fraction = overlap / h;

    // Use kernel-weighted contribution
    rest_density * volume_fraction * cubic_spline_kernel(sdf_distance, h)
}

// =============================================================================
// VELOCITY CONSTRAINTS
// =============================================================================

/// Apply slip condition to velocity
pub fn apply_slip_condition(
    velocity: [f32; 3],
    normal: [f32; 3],
    boundary_velocity: [f32; 3],
    slip: SlipCondition,
) -> [f32; 3] {
    // Relative velocity
    let rel_vel = [
        velocity[0] - boundary_velocity[0],
        velocity[1] - boundary_velocity[1],
        velocity[2] - boundary_velocity[2],
    ];

    // Normal component
    let v_dot_n = rel_vel[0] * normal[0] + rel_vel[1] * normal[1] + rel_vel[2] * normal[2];
    let normal_vel = [
        v_dot_n * normal[0],
        v_dot_n * normal[1],
        v_dot_n * normal[2],
    ];

    // Tangential component
    let tangent_vel = [
        rel_vel[0] - normal_vel[0],
        rel_vel[1] - normal_vel[1],
        rel_vel[2] - normal_vel[2],
    ];

    match slip {
        SlipCondition::NoSlip => {
            // Zero relative velocity
            boundary_velocity
        }
        SlipCondition::FreeSlip => {
            // Keep tangential, zero normal (reflect if moving into boundary)
            if v_dot_n < 0.0 {
                // Moving into boundary - reflect
                [
                    boundary_velocity[0] + tangent_vel[0],
                    boundary_velocity[1] + tangent_vel[1],
                    boundary_velocity[2] + tangent_vel[2],
                ]
            } else {
                // Moving away - keep original
                velocity
            }
        }
        SlipCondition::PartialSlip(alpha) => {
            // Blend between no-slip and free-slip
            let clamped = alpha.clamp(0.0, 1.0);

            if v_dot_n < 0.0 {
                let free_slip_vel = [
                    boundary_velocity[0] + tangent_vel[0],
                    boundary_velocity[1] + tangent_vel[1],
                    boundary_velocity[2] + tangent_vel[2],
                ];

                [
                    clamped * boundary_velocity[0] + (1.0 - clamped) * free_slip_vel[0],
                    clamped * boundary_velocity[1] + (1.0 - clamped) * free_slip_vel[1],
                    clamped * boundary_velocity[2] + (1.0 - clamped) * free_slip_vel[2],
                ]
            } else {
                velocity
            }
        }
    }
}

/// Apply friction to tangential velocity
pub fn apply_friction(
    velocity: [f32; 3],
    normal: [f32; 3],
    boundary_velocity: [f32; 3],
    friction: f32,
    dt: f32,
) -> [f32; 3] {
    // Relative velocity
    let rel_vel = [
        velocity[0] - boundary_velocity[0],
        velocity[1] - boundary_velocity[1],
        velocity[2] - boundary_velocity[2],
    ];

    // Normal component
    let v_dot_n = rel_vel[0] * normal[0] + rel_vel[1] * normal[1] + rel_vel[2] * normal[2];

    // Tangential component
    let tangent_vel = [
        rel_vel[0] - v_dot_n * normal[0],
        rel_vel[1] - v_dot_n * normal[1],
        rel_vel[2] - v_dot_n * normal[2],
    ];

    let tangent_speed = (tangent_vel[0] * tangent_vel[0]
        + tangent_vel[1] * tangent_vel[1]
        + tangent_vel[2] * tangent_vel[2])
        .sqrt();

    if tangent_speed < MIN_DISTANCE {
        return velocity;
    }

    // Friction deceleration (Coulomb model)
    let normal_force = v_dot_n.abs(); // Simplified - proportional to normal velocity
    let friction_decel = friction * normal_force * 10.0; // Scale factor for effect
    let velocity_reduction = (friction_decel * dt).min(tangent_speed);

    let factor = 1.0 - velocity_reduction / tangent_speed;

    [
        boundary_velocity[0] + v_dot_n * normal[0] + factor * tangent_vel[0],
        boundary_velocity[1] + v_dot_n * normal[1] + factor * tangent_vel[1],
        boundary_velocity[2] + v_dot_n * normal[2] + factor * tangent_vel[2],
    ]
}

// =============================================================================
// BOUNDARY FORCE COMPUTATION
// =============================================================================

/// Compute pressure force from boundary particles
///
/// Uses Akinci formulation for solid-fluid pressure coupling
pub fn boundary_pressure_force(
    fluid_pos: [f32; 3],
    fluid_density: f32,
    fluid_pressure: f32,
    boundary_particles: &[BoundaryParticle],
    rest_density: f32,
    h: f32,
) -> [f32; 3] {
    let mut force = [0.0f32; 3];

    for bp in boundary_particles {
        let dx = fluid_pos[0] - bp.position[0];
        let dy = fluid_pos[1] - bp.position[1];
        let dz = fluid_pos[2] - bp.position[2];
        let r = (dx * dx + dy * dy + dz * dz).sqrt();

        if r > MIN_DISTANCE && r < h {
            let grad_w = cubic_spline_gradient(r, h) / r;

            // Boundary pressure (mirrored from fluid)
            let boundary_pressure = fluid_pressure;

            // Pressure force contribution
            let pressure_term = fluid_pressure / (fluid_density * fluid_density)
                + boundary_pressure / (rest_density * rest_density);

            let scale = -rest_density * bp.volume * pressure_term * grad_w;

            force[0] += scale * dx;
            force[1] += scale * dy;
            force[2] += scale * dz;
        }
    }

    force
}

// =============================================================================
// BOUNDARY SAMPLER
// =============================================================================

/// Configuration for sparse Akinci sampling
#[derive(Clone, Debug)]
pub struct SparseAkinciConfig {
    /// Spacing between boundary particles (relative to h)
    pub spacing: f32,
    /// Only sample densely at corners/edges
    pub only_at_corners: bool,
    /// Refine sampling where fluid is nearby
    pub adaptive_sampling: bool,
    /// Maximum boundary particles per object
    pub max_particles: usize,
}

impl Default for SparseAkinciConfig {
    fn default() -> Self {
        Self {
            spacing: 0.5,
            only_at_corners: false,
            adaptive_sampling: true,
            max_particles: 10000,
        }
    }
}

/// Sample a plane with boundary particles
pub fn sample_plane(
    center: [f32; 3],
    normal: [f32; 3],
    size: [f32; 2],
    h: f32,
    config: &SparseAkinciConfig,
) -> Vec<BoundaryParticle> {
    let mut particles = Vec::new();
    let spacing = h * config.spacing;

    // Compute tangent vectors
    let up = if normal[1].abs() < 0.9 {
        [0.0, 1.0, 0.0]
    } else {
        [1.0, 0.0, 0.0]
    };

    // tangent1 = up × normal
    let t1 = [
        up[1] * normal[2] - up[2] * normal[1],
        up[2] * normal[0] - up[0] * normal[2],
        up[0] * normal[1] - up[1] * normal[0],
    ];
    let t1_len = (t1[0] * t1[0] + t1[1] * t1[1] + t1[2] * t1[2]).sqrt();
    let t1 = [t1[0] / t1_len, t1[1] / t1_len, t1[2] / t1_len];

    // tangent2 = normal × tangent1
    let t2 = [
        normal[1] * t1[2] - normal[2] * t1[1],
        normal[2] * t1[0] - normal[0] * t1[2],
        normal[0] * t1[1] - normal[1] * t1[0],
    ];

    // Sample grid
    let nx = (size[0] / spacing).ceil() as i32;
    let ny = (size[1] / spacing).ceil() as i32;

    for i in -nx..=nx {
        for j in -ny..=ny {
            let u = i as f32 * spacing;
            let v = j as f32 * spacing;

            if u.abs() <= size[0] * 0.5 && v.abs() <= size[1] * 0.5 {
                let pos = [
                    center[0] + u * t1[0] + v * t2[0],
                    center[1] + u * t1[1] + v * t2[1],
                    center[2] + u * t1[2] + v * t2[2],
                ];

                let particle =
                    BoundaryParticle::new(pos, normal).with_volume(spacing * spacing * h);

                particles.push(particle);

                if particles.len() >= config.max_particles {
                    return particles;
                }
            }
        }
    }

    particles
}

/// Sample a box with boundary particles
pub fn sample_box(
    min: [f32; 3],
    max: [f32; 3],
    h: f32,
    config: &SparseAkinciConfig,
) -> Vec<BoundaryParticle> {
    let mut particles = Vec::new();

    let center = [
        (min[0] + max[0]) * 0.5,
        (min[1] + max[1]) * 0.5,
        (min[2] + max[2]) * 0.5,
    ];

    let size = [max[0] - min[0], max[1] - min[1], max[2] - min[2]];

    // Bottom face (-Y)
    let bottom_center = [center[0], min[1], center[2]];
    particles.extend(sample_plane(
        bottom_center,
        [0.0, 1.0, 0.0],
        [size[0], size[2]],
        h,
        config,
    ));

    // Top face (+Y)
    let top_center = [center[0], max[1], center[2]];
    particles.extend(sample_plane(
        top_center,
        [0.0, -1.0, 0.0],
        [size[0], size[2]],
        h,
        config,
    ));

    // Left face (-X)
    let left_center = [min[0], center[1], center[2]];
    particles.extend(sample_plane(
        left_center,
        [1.0, 0.0, 0.0],
        [size[1], size[2]],
        h,
        config,
    ));

    // Right face (+X)
    let right_center = [max[0], center[1], center[2]];
    particles.extend(sample_plane(
        right_center,
        [-1.0, 0.0, 0.0],
        [size[1], size[2]],
        h,
        config,
    ));

    // Front face (-Z)
    let front_center = [center[0], center[1], min[2]];
    particles.extend(sample_plane(
        front_center,
        [0.0, 0.0, 1.0],
        [size[0], size[1]],
        h,
        config,
    ));

    // Back face (+Z)
    let back_center = [center[0], center[1], max[2]];
    particles.extend(sample_plane(
        back_center,
        [0.0, 0.0, -1.0],
        [size[0], size[1]],
        h,
        config,
    ));

    particles
}

/// Sample a sphere with boundary particles
pub fn sample_sphere(
    center: [f32; 3],
    radius: f32,
    h: f32,
    config: &SparseAkinciConfig,
) -> Vec<BoundaryParticle> {
    let mut particles = Vec::new();
    let spacing = h * config.spacing;

    // Fibonacci sphere sampling for uniform distribution
    let n = ((4.0 * std::f32::consts::PI * radius * radius) / (spacing * spacing)) as usize;
    let n = n.min(config.max_particles);

    let golden_ratio = (1.0 + 5.0_f32.sqrt()) * 0.5;

    for i in 0..n {
        let theta = 2.0 * std::f32::consts::PI * (i as f32) / golden_ratio;
        let phi = (1.0 - 2.0 * (i as f32 + 0.5) / n as f32).acos();

        let x = phi.sin() * theta.cos();
        let y = phi.sin() * theta.sin();
        let z = phi.cos();

        let pos = [
            center[0] + radius * x,
            center[1] + radius * y,
            center[2] + radius * z,
        ];

        // Normal points inward (into fluid)
        let normal = [-x, -y, -z];

        let particle = BoundaryParticle::new(pos, normal).with_volume(spacing * spacing * h);

        particles.push(particle);
    }

    particles
}

// =============================================================================
// BOUNDARY SYSTEM
// =============================================================================

/// Boundary handling system
pub struct BoundarySystem {
    config: BoundaryConfig,
    particles: Vec<BoundaryParticle>,
    stats: BoundaryStats,
}

/// Statistics for boundary system
#[derive(Clone, Default)]
pub struct BoundaryStats {
    /// Total boundary particles
    pub particle_count: usize,
    /// Average density contribution
    pub avg_density_contrib: f32,
    /// Maximum friction applied
    pub max_friction: f32,
    /// Particles near fluid
    pub active_particles: usize,
}

impl BoundarySystem {
    /// Create a new boundary system
    pub fn new(config: BoundaryConfig) -> Self {
        Self {
            config,
            particles: Vec::new(),
            stats: BoundaryStats::default(),
        }
    }

    /// Create with default config
    pub fn default_config() -> Self {
        Self::new(BoundaryConfig::default())
    }

    /// Get current config
    pub fn config(&self) -> &BoundaryConfig {
        &self.config
    }

    /// Get mutable config
    pub fn config_mut(&mut self) -> &mut BoundaryConfig {
        &mut self.config
    }

    /// Add boundary particles
    pub fn add_particles(&mut self, particles: Vec<BoundaryParticle>) {
        self.particles.extend(particles);
        self.stats.particle_count = self.particles.len();
    }

    /// Add a plane boundary
    pub fn add_plane(&mut self, center: [f32; 3], normal: [f32; 3], size: [f32; 2]) {
        let sparse_config = SparseAkinciConfig {
            spacing: self.config.particle_spacing,
            ..Default::default()
        };
        let particles = sample_plane(center, normal, size, self.config.h, &sparse_config);
        self.add_particles(particles);
    }

    /// Add a box boundary
    pub fn add_box(&mut self, min: [f32; 3], max: [f32; 3]) {
        let sparse_config = SparseAkinciConfig {
            spacing: self.config.particle_spacing,
            ..Default::default()
        };
        let particles = sample_box(min, max, self.config.h, &sparse_config);
        self.add_particles(particles);
    }

    /// Add a sphere boundary
    pub fn add_sphere(&mut self, center: [f32; 3], radius: f32) {
        let sparse_config = SparseAkinciConfig {
            spacing: self.config.particle_spacing,
            ..Default::default()
        };
        let particles = sample_sphere(center, radius, self.config.h, &sparse_config);
        self.add_particles(particles);
    }

    /// Clear all boundary particles
    pub fn clear(&mut self) {
        self.particles.clear();
        self.stats.particle_count = 0;
    }

    /// Get all boundary particles
    pub fn particles(&self) -> &[BoundaryParticle] {
        &self.particles
    }

    /// Get boundary statistics
    pub fn stats(&self) -> &BoundaryStats {
        &self.stats
    }

    /// Compute boundary density contribution for a fluid particle
    pub fn compute_density(&self, fluid_pos: [f32; 3]) -> f32 {
        match self.config.method {
            BoundaryMethod::AkinciOnly
            | BoundaryMethod::Hybrid {
                sdf_for_density: false,
                ..
            } => boundary_density_akinci(
                fluid_pos,
                &self.particles,
                self.config.rest_density,
                self.config.h,
            ),
            BoundaryMethod::SdfOnly
            | BoundaryMethod::Hybrid {
                sdf_for_density: true,
                ..
            } => {
                // Would require SDF field - return 0 for now (needs SDF integration)
                0.0
            }
        }
    }

    /// Apply boundary constraints to velocity
    pub fn apply_constraints(&self, position: [f32; 3], velocity: [f32; 3], dt: f32) -> [f32; 3] {
        let mut result = velocity;

        // Find nearest boundary particle
        let mut nearest_dist = f32::MAX;
        let mut nearest_bp: Option<&BoundaryParticle> = None;

        for bp in &self.particles {
            let dx = position[0] - bp.position[0];
            let dy = position[1] - bp.position[1];
            let dz = position[2] - bp.position[2];
            let dist = (dx * dx + dy * dy + dz * dz).sqrt();

            if dist < nearest_dist && dist < self.config.h {
                nearest_dist = dist;
                nearest_bp = Some(bp);
            }
        }

        if let Some(bp) = nearest_bp {
            // Apply slip condition
            result =
                apply_slip_condition(result, bp.normal, bp.velocity, self.config.slip_condition);

            // Apply friction
            let friction = bp.friction.max(self.config.friction);
            result = apply_friction(result, bp.normal, bp.velocity, friction, dt);
        }

        result
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_boundary_particle_default() {
        let bp = BoundaryParticle::default();
        assert_eq!(bp.position, [0.0, 0.0, 0.0]);
        assert_eq!(bp.volume, 1.0);
        assert_eq!(bp.normal, [0.0, 1.0, 0.0]);
        assert_eq!(bp.friction, 0.5);
        assert_eq!(bp.boundary_type, 0);
    }

    #[test]
    fn test_boundary_particle_builder() {
        let bp = BoundaryParticle::new([1.0, 2.0, 3.0], [0.0, 0.0, 1.0])
            .with_volume(2.0)
            .with_friction(0.8)
            .with_object_id(5);

        assert_eq!(bp.position, [1.0, 2.0, 3.0]);
        assert_eq!(bp.normal, [0.0, 0.0, 1.0]);
        assert_eq!(bp.volume, 2.0);
        assert_eq!(bp.friction, 0.8);
        assert_eq!(bp.object_id, 5);
    }

    #[test]
    fn test_boundary_particle_dynamic() {
        let bp = BoundaryParticle::default().as_dynamic([1.0, 0.0, 0.0]);

        assert_eq!(bp.boundary_type, 1);
        assert_eq!(bp.velocity, [1.0, 0.0, 0.0]);
    }

    #[test]
    fn test_boundary_particle_size() {
        // Verify struct size (14 floats * 4 bytes = 56 bytes)
        // position[3] + volume + normal[3] + friction + object_id + boundary_type + velocity[3] + _pad
        assert_eq!(std::mem::size_of::<BoundaryParticle>(), 56);
    }

    #[test]
    fn test_boundary_method_default() {
        let method = BoundaryMethod::default();
        assert!(matches!(
            method,
            BoundaryMethod::Hybrid {
                sdf_for_density: true,
                particles_for_friction: true
            }
        ));
    }

    #[test]
    fn test_slip_condition_factor() {
        assert!((SlipCondition::NoSlip.factor() - 1.0).abs() < 1e-6);
        assert!((SlipCondition::FreeSlip.factor() - 0.0).abs() < 1e-6);
        assert!((SlipCondition::PartialSlip(0.5).factor() - 0.5).abs() < 1e-6);
    }

    #[test]
    fn test_cubic_spline_kernel_normalization() {
        let h = 0.1;
        // Kernel should be maximum at r = 0
        let w_at_0 = cubic_spline_kernel(0.0, h);
        let w_at_half = cubic_spline_kernel(h * 0.5, h);
        let w_at_h = cubic_spline_kernel(h, h);

        assert!(w_at_0 > w_at_half);
        assert!(w_at_half > w_at_h);
        assert!((w_at_h).abs() < 1e-6); // Should be zero at boundary
    }

    #[test]
    fn test_cubic_spline_kernel_outside() {
        let h = 0.1;
        assert!((cubic_spline_kernel(h + 0.01, h)).abs() < 1e-6);
        assert!((cubic_spline_kernel(h * 2.0, h)).abs() < 1e-6);
    }

    #[test]
    fn test_boundary_density_akinci() {
        let fluid_pos = [0.0, 0.05, 0.0];
        let h: f32 = 0.1;

        let bp = BoundaryParticle::new([0.0, 0.0, 0.0], [0.0, 1.0, 0.0]).with_volume(h.powi(3));

        let particles = vec![bp];
        let density = boundary_density_akinci(fluid_pos, &particles, 1000.0, h);

        assert!(density > 0.0);
        assert!(density < 1000.0);
    }

    #[test]
    fn test_boundary_density_sdf() {
        let h: f32 = 0.1;

        // Inside boundary
        let density_inside = boundary_density_sdf(-0.05, 1000.0, h);
        assert_eq!(density_inside, 1000.0);

        // Far outside
        let density_far = boundary_density_sdf(h * 2.0, 1000.0, h);
        assert!((density_far).abs() < 1e-6);

        // At boundary (sdf = 0) should be full density
        let density_at_boundary = boundary_density_sdf(0.0, 1000.0, h);
        assert_eq!(density_at_boundary, 1000.0);

        // Near boundary (sdf slightly positive)
        let density_near = boundary_density_sdf(h * 0.9, 1000.0, h);
        assert!(density_near >= 0.0); // Kernel-weighted, positive contribution
    }

    #[test]
    fn test_apply_slip_no_slip() {
        let velocity = [1.0, 0.0, 0.0];
        let normal = [0.0, 1.0, 0.0];
        let boundary_vel = [0.0, 0.0, 0.0];

        let result = apply_slip_condition(velocity, normal, boundary_vel, SlipCondition::NoSlip);

        // Should match boundary velocity (zero)
        assert!((result[0]).abs() < 1e-6);
        assert!((result[1]).abs() < 1e-6);
        assert!((result[2]).abs() < 1e-6);
    }

    #[test]
    fn test_apply_slip_free_slip() {
        let velocity = [1.0, -1.0, 0.0]; // Moving into boundary
        let normal = [0.0, 1.0, 0.0];
        let boundary_vel = [0.0, 0.0, 0.0];

        let result = apply_slip_condition(velocity, normal, boundary_vel, SlipCondition::FreeSlip);

        // Should keep tangential (x), remove normal (y)
        assert!((result[0] - 1.0).abs() < 1e-6);
        assert!((result[1]).abs() < 1e-6);
        assert!((result[2]).abs() < 1e-6);
    }

    #[test]
    fn test_apply_friction() {
        // Velocity with normal component (impacting surface + sliding)
        let velocity = [1.0, -0.5, 0.0]; // Sliding + pressing into surface
        let normal = [0.0, 1.0, 0.0];
        let boundary_vel = [0.0, 0.0, 0.0];
        let friction = 0.5;
        let dt = 0.016;

        let result = apply_friction(velocity, normal, boundary_vel, friction, dt);

        // Friction should reduce tangential velocity (x component)
        // With normal force from y component, friction acts on x velocity
        assert!(
            result[0] < velocity[0],
            "Friction should reduce tangential velocity: {} >= {}",
            result[0],
            velocity[0]
        );
        assert!(result[0] >= 0.0);
    }

    #[test]
    fn test_sample_plane() {
        let config = SparseAkinciConfig::default();
        let particles = sample_plane([0.0, 0.0, 0.0], [0.0, 1.0, 0.0], [1.0, 1.0], 0.1, &config);

        assert!(!particles.is_empty());

        // All particles should have same normal
        for p in &particles {
            assert_eq!(p.normal, [0.0, 1.0, 0.0]);
        }
    }

    #[test]
    fn test_sample_box() {
        let config = SparseAkinciConfig::default();
        let particles = sample_box([0.0, 0.0, 0.0], [1.0, 1.0, 1.0], 0.1, &config);

        assert!(!particles.is_empty());
        // Should have particles from all 6 faces
        assert!(particles.len() > 100);
    }

    #[test]
    fn test_sample_sphere() {
        let config = SparseAkinciConfig::default();
        let particles = sample_sphere([0.0, 0.0, 0.0], 1.0, 0.1, &config);

        assert!(!particles.is_empty());

        // All particles should be on sphere surface
        for p in &particles {
            let r = (p.position[0].powi(2) + p.position[1].powi(2) + p.position[2].powi(2)).sqrt();
            assert!((r - 1.0).abs() < 0.1);
        }
    }

    #[test]
    fn test_boundary_system_creation() {
        let system = BoundarySystem::new(BoundaryConfig::default());
        assert!(system.particles().is_empty());
    }

    #[test]
    fn test_boundary_system_add_plane() {
        let mut system = BoundarySystem::new(BoundaryConfig::default());
        system.add_plane([0.0, 0.0, 0.0], [0.0, 1.0, 0.0], [2.0, 2.0]);

        assert!(!system.particles().is_empty());
        assert_eq!(system.stats().particle_count, system.particles().len());
    }

    #[test]
    fn test_boundary_system_add_box() {
        let mut system = BoundarySystem::new(BoundaryConfig::default());
        system.add_box([0.0, 0.0, 0.0], [1.0, 1.0, 1.0]);

        assert!(!system.particles().is_empty());
    }

    #[test]
    fn test_boundary_system_add_sphere() {
        let mut system = BoundarySystem::new(BoundaryConfig::default());
        system.add_sphere([0.0, 0.0, 0.0], 1.0);

        assert!(!system.particles().is_empty());
    }

    #[test]
    fn test_boundary_system_clear() {
        let mut system = BoundarySystem::new(BoundaryConfig::default());
        system.add_box([0.0, 0.0, 0.0], [1.0, 1.0, 1.0]);
        assert!(!system.particles().is_empty());

        system.clear();
        assert!(system.particles().is_empty());
        assert_eq!(system.stats().particle_count, 0);
    }

    #[test]
    fn test_boundary_config_presets() {
        let static_config = BoundaryConfig::static_walls();
        assert!(matches!(static_config.method, BoundaryMethod::SdfOnly));

        let smooth_config = BoundaryConfig::smooth_surface();
        assert!(matches!(
            smooth_config.slip_condition,
            SlipCondition::FreeSlip
        ));

        let rough_config = BoundaryConfig::rough_surface();
        assert!(rough_config.friction > 0.5);

        let dynamic_config = BoundaryConfig::dynamic_object();
        assert!(matches!(dynamic_config.method, BoundaryMethod::AkinciOnly));
    }

    #[test]
    fn test_boundary_system_compute_density() {
        let mut system = BoundarySystem::new(BoundaryConfig {
            method: BoundaryMethod::AkinciOnly,
            h: 0.1,
            ..Default::default()
        });

        system.add_plane([0.0, 0.0, 0.0], [0.0, 1.0, 0.0], [2.0, 2.0]);

        // Fluid particle near boundary should have density contribution
        let density = system.compute_density([0.0, 0.05, 0.0]);
        assert!(density > 0.0);

        // Fluid particle far from boundary should have no contribution
        let density_far = system.compute_density([0.0, 1.0, 0.0]);
        assert!((density_far).abs() < 1e-6);
    }

    #[test]
    fn test_boundary_pressure_force() {
        let bp = BoundaryParticle::new([0.0, 0.0, 0.0], [0.0, 1.0, 0.0]).with_volume(0.001);

        let particles = vec![bp];
        let force = boundary_pressure_force(
            [0.0, 0.05, 0.0], // fluid position
            1000.0,           // fluid density
            100.0,            // fluid pressure
            &particles,
            1000.0, // rest density
            0.1,    // h
        );

        // Force should push fluid away from boundary
        assert!(force[1] > 0.0);
    }

    // =========================================================================
    // Mutation-killing tests
    // =========================================================================

    #[test]
    fn test_as_kinematic_sets_boundary_type() {
        // Catches: replace as_kinematic -> Self with Default::default()
        let bp = BoundaryParticle::new([1.0, 2.0, 3.0], [0.0, 0.0, 1.0])
            .with_volume(2.5)
            .as_kinematic();
        assert_eq!(bp.boundary_type, 2, "as_kinematic must set boundary_type=2");
        // These would be wrong if Default::default() was returned
        assert_eq!(bp.position, [1.0, 2.0, 3.0]);
        assert_eq!(bp.volume, 2.5);
    }

    #[test]
    fn test_static_walls_all_fields() {
        let c = BoundaryConfig::static_walls();
        assert!(
            matches!(c.slip_condition, SlipCondition::NoSlip),
            "static_walls must use NoSlip"
        );
        assert!(
            (c.friction - 0.8).abs() < 1e-6,
            "static_walls friction should be 0.8, got {}",
            c.friction
        );
    }

    #[test]
    fn test_smooth_surface_all_fields() {
        let c = BoundaryConfig::smooth_surface();
        assert!(
            matches!(
                c.method,
                BoundaryMethod::Hybrid {
                    sdf_for_density: true,
                    particles_for_friction: false
                }
            ),
            "smooth_surface method mismatch"
        );
        assert!(
            (c.friction - 0.1).abs() < 1e-6,
            "smooth_surface friction should be 0.1, got {}",
            c.friction
        );
        assert!(
            (c.restitution - 0.5).abs() < 1e-6,
            "smooth_surface restitution should be 0.5, got {}",
            c.restitution
        );
    }

    #[test]
    fn test_rough_surface_all_fields() {
        let c = BoundaryConfig::rough_surface();
        assert!(
            matches!(
                c.method,
                BoundaryMethod::Hybrid {
                    sdf_for_density: true,
                    particles_for_friction: true
                }
            ),
            "rough_surface method mismatch"
        );
        assert!(
            matches!(c.slip_condition, SlipCondition::NoSlip),
            "rough_surface must use NoSlip"
        );
        assert!(
            (c.restitution - 0.1).abs() < 1e-6,
            "rough_surface restitution should be 0.1, got {}",
            c.restitution
        );
    }

    #[test]
    fn test_dynamic_object_all_fields() {
        let c = BoundaryConfig::dynamic_object();
        if let SlipCondition::PartialSlip(f) = c.slip_condition {
            assert!(
                (f - 0.5).abs() < 1e-6,
                "dynamic_object partial slip should be 0.5, got {}",
                f
            );
        } else {
            panic!(
                "dynamic_object must use PartialSlip, got {:?}",
                c.slip_condition
            );
        }
        assert!(
            (c.friction - 0.5).abs() < 1e-6,
            "dynamic_object friction should be 0.5, got {}",
            c.friction
        );
        assert!(
            c.adaptive_sampling,
            "dynamic_object must enable adaptive_sampling"
        );
    }

    #[test]
    fn test_cubic_spline_kernel_exact_first_branch() {
        // Catches: sigma /→*, /→%, *→+, *→/ mutations (lines 279)
        let h = 0.1_f32;
        let r = 0.025_f32; // q = 0.25, first branch (q <= 0.5)
        let sigma = 8.0 / (std::f32::consts::PI * h.powi(3));
        let q = r / h;
        let expected = sigma * (6.0 * q.powi(3) - 6.0 * q.powi(2) + 1.0);

        let actual = cubic_spline_kernel(r, h);
        let rel_err = (actual - expected).abs() / expected.abs();
        assert!(
            rel_err < 1e-5,
            "kernel({},{})={}, expected={}, rel_err={}",
            r,
            h,
            actual,
            expected,
            rel_err
        );
    }

    #[test]
    fn test_cubic_spline_kernel_exact_second_branch() {
        // Catches: sigma * 2.0 → sigma / 2.0 mutation (line 284)
        let h = 0.1_f32;
        let r = 0.08_f32; // q = 0.8, second branch
        let sigma = 8.0 / (std::f32::consts::PI * h.powi(3));
        let q = r / h;
        let expected = sigma * 2.0 * (1.0 - q).powi(3);

        let actual = cubic_spline_kernel(r, h);
        let rel_err = (actual - expected).abs() / expected.abs();
        assert!(
            rel_err < 1e-4,
            "kernel({},{})={}, expected={}, rel_err={}",
            r,
            h,
            actual,
            expected,
            rel_err
        );
    }

    #[test]
    fn test_cubic_spline_gradient_exact_first_branch() {
        // Catches: q=/→*, sigma /→*, *→+, branch <=→>, and /h→*h mutations
        let h = 0.1_f32;
        let r = 0.03_f32; // q = 0.3, well inside first branch
        let sigma = 8.0 / (std::f32::consts::PI * h.powi(3));
        let q = r / h;
        let expected = sigma * (18.0 * q * q - 12.0 * q) / h;

        let actual = cubic_spline_gradient(r, h);
        let rel_err = (actual - expected).abs() / expected.abs();
        assert!(
            rel_err < 1e-4,
            "gradient({},{})={}, expected={}, rel_err={}",
            r,
            h,
            actual,
            expected,
            rel_err
        );
    }

    #[test]
    fn test_cubic_spline_gradient_exact_second_branch() {
        // Catches: second branch gradient computation mutations
        let h = 0.1_f32;
        let r = 0.08_f32; // q = 0.8, second branch
        let sigma = 8.0 / (std::f32::consts::PI * h.powi(3));
        let q = r / h;
        let expected = -sigma * 6.0 * (1.0 - q).powi(2) / h;

        let actual = cubic_spline_gradient(r, h);
        let rel_err = (actual - expected).abs() / expected.abs();
        assert!(
            rel_err < 1e-4,
            "gradient({},{})={}, expected={}, rel_err={}",
            r,
            h,
            actual,
            expected,
            rel_err
        );
    }

    #[test]
    fn test_cubic_spline_gradient_at_min_distance_nonzero() {
        // Catches: < → == and < → <= mutations on the MIN_DISTANCE check
        // At r = MIN_DISTANCE exactly, original continues (< is false)
        // but == or <= mutations would early-return 0.0
        let h = 0.1_f32;
        let result = cubic_spline_gradient(MIN_DISTANCE, h);
        assert!(
            result.abs() > 0.0,
            "gradient at MIN_DISTANCE should be nonzero, got {}",
            result
        );
    }
}
