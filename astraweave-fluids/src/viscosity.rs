//! # Research-Grade Viscosity Models for SPH
//!
//! This module implements physically-accurate viscosity models for SPH fluid simulation:
//!
//! ## Explicit Methods
//! - **Morris Viscosity**: Standard SPH viscosity formulation (Morris et al. 1997)
//! - **XSPH Viscosity**: Simplified artificial viscosity (Monaghan 1992)
//!
//! ## Implicit Methods
//! - **Matrix-Free Jacobi**: GPU-efficient implicit solver (Weiler et al. 2018)
//!
//! ## Non-Newtonian Models
//! - **Carreau Model**: Shear-thinning/thickening fluids
//! - **Power Law**: Simple power-law viscosity
//! - **Cross Model**: Alternative to Carreau for polymer solutions
//!
//! ## Temperature Dependence
//! - **Arrhenius Model**: Exponential temperature sensitivity
//! - **VTF Model**: Vogel-Tammann-Fulcher equation
//!
//! # References
//! - Morris, Fox & Zhu (1997) "Modeling Low Reynolds Number Incompressible Flows Using SPH"
//! - Weiler et al. (2018) "A Physically Consistent Implicit Viscosity Solver for SPH Fluids"
//! - Peer et al. (2015) "An Implicit SPH Formulation for Incompressible Linearly Elastic Solids"

use crate::research::{ResearchParticle, ShearRateMethod, ViscositySolver};

// ============================================================================
// KERNEL FUNCTIONS
// ============================================================================

/// Cubic spline kernel W(r, h) for SPH
///
/// # Arguments
/// * `r` - Distance between particles
/// * `h` - Smoothing radius
#[inline]
pub fn kernel_w(r: f32, h: f32) -> f32 {
    if r >= h {
        return 0.0;
    }

    let q = r / h;
    let sigma = 8.0 / (std::f32::consts::PI * h * h * h);

    if q <= 0.5 {
        sigma * (6.0 * q * q * q - 6.0 * q * q + 1.0)
    } else {
        let one_minus_q = 1.0 - q;
        sigma * 2.0 * one_minus_q * one_minus_q * one_minus_q
    }
}

/// Gradient of cubic spline kernel ∇W(r, h)
///
/// Returns magnitude of gradient (always positive, direction is -r̂)
/// The gradient points toward higher density (center)
#[inline]
pub fn kernel_gradient_mag(r: f32, h: f32) -> f32 {
    if r >= h || r < 1e-10 {
        return 0.0;
    }

    let q = r / h;
    let sigma = 48.0 / (std::f32::consts::PI * h * h * h * h);

    // The derivative of W with respect to r is negative (W decreases with r)
    // but we want magnitude, so we take absolute value
    if q <= 0.5 {
        // dW/dr = sigma * q * (3q - 2) / h
        // At q=0.25: 0.25 * (0.75-2) = -0.3125 (negative)
        sigma * q * (2.0 - 3.0 * q) // Flip sign to get positive magnitude
    } else {
        let one_minus_q = 1.0 - q;
        sigma * one_minus_q * one_minus_q // Already positive for q > 0.5
    }
}

/// Laplacian of kernel ∇²W(r, h) for viscosity computation
///
/// Uses the second derivative of the cubic spline
#[inline]
pub fn kernel_laplacian(r: f32, h: f32) -> f32 {
    if r >= h {
        return 0.0;
    }

    let q = r / h;
    let sigma = 48.0 / (std::f32::consts::PI * h * h * h * h * h);

    if q <= 0.5 {
        sigma * (6.0 * q - 2.0)
    } else {
        let one_minus_q = 1.0 - q;
        sigma * 2.0 * one_minus_q
    }
}

// ============================================================================
// VISCOSITY CONFIGURATION
// ============================================================================

/// Configuration for viscosity computation
#[derive(Clone, Debug)]
pub struct ViscosityConfig {
    /// Solver method
    pub solver: ViscositySolver,
    /// Base dynamic viscosity (Pa·s)
    pub base_viscosity: f32,
    /// Smoothing radius h
    pub smoothing_radius: f32,
    /// Time step dt
    pub dt: f32,
    /// Particle mass
    pub particle_mass: f32,

    // Implicit solver settings
    /// Maximum Jacobi iterations
    pub max_iterations: u32,
    /// Convergence tolerance
    pub tolerance: f32,
    /// SOR relaxation factor (0.5-1.0)
    pub omega: f32,

    // Non-Newtonian settings
    /// Enable non-Newtonian behavior
    pub enable_non_newtonian: bool,
    /// Non-Newtonian model
    pub non_newtonian_model: NonNewtonianModel,
    /// Shear rate estimation method
    pub shear_rate_method: ShearRateMethod,

    // Temperature settings
    /// Enable temperature-dependent viscosity
    pub enable_temperature: bool,
    /// Temperature model
    pub temperature_model: TemperatureModel,
}

impl Default for ViscosityConfig {
    fn default() -> Self {
        Self {
            solver: ViscositySolver::Morris,
            base_viscosity: 0.001, // Water at 20°C
            smoothing_radius: 1.2,
            dt: 1.0 / 60.0,
            particle_mass: 1.0,

            max_iterations: 5,
            tolerance: 1e-4,
            omega: 0.7,

            enable_non_newtonian: false,
            non_newtonian_model: NonNewtonianModel::default(),
            shear_rate_method: ShearRateMethod::VorticityBased,

            enable_temperature: false,
            temperature_model: TemperatureModel::default(),
        }
    }
}

impl ViscosityConfig {
    /// Create config for water at standard conditions
    pub fn water() -> Self {
        Self {
            base_viscosity: 0.001,
            ..Default::default()
        }
    }

    /// Create config for oil
    pub fn oil() -> Self {
        Self {
            base_viscosity: 0.05,
            solver: ViscositySolver::Morris,
            ..Default::default()
        }
    }

    /// Create config for honey (high viscosity, requires implicit solver)
    pub fn honey() -> Self {
        Self {
            base_viscosity: 5.0,
            solver: ViscositySolver::ImplicitJacobi,
            max_iterations: 15,
            ..Default::default()
        }
    }

    /// Create config for non-Newtonian fluid (e.g., ketchup)
    pub fn shear_thinning() -> Self {
        Self {
            base_viscosity: 0.5,
            solver: ViscositySolver::Morris,
            enable_non_newtonian: true,
            non_newtonian_model: NonNewtonianModel::carreau(0.5, 0.3, 1.0, 0.01),
            ..Default::default()
        }
    }

    /// Create config for cornstarch (shear-thickening)
    pub fn shear_thickening() -> Self {
        Self {
            base_viscosity: 0.1,
            solver: ViscositySolver::ImplicitJacobi,
            enable_non_newtonian: true,
            non_newtonian_model: NonNewtonianModel::power_law(0.1, 1.5),
            max_iterations: 10,
            ..Default::default()
        }
    }
}

// ============================================================================
// NON-NEWTONIAN MODELS
// ============================================================================

/// Non-Newtonian viscosity model selection
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[non_exhaustive]
pub enum NonNewtonianType {
    /// Newtonian (constant viscosity)
    #[default]
    Newtonian,
    /// Power law: μ = K γ̇^(n-1)
    PowerLaw,
    /// Carreau model: μ = μ∞ + (μ₀ - μ∞)(1 + (λγ̇)²)^((n-1)/2)
    Carreau,
    /// Cross model: μ = μ∞ + (μ₀ - μ∞)/(1 + (λγ̇)^m)
    Cross,
    /// Bingham plastic: μ = μ₀ + τ_y/γ̇
    Bingham,
}

/// Parameters for non-Newtonian viscosity models
#[derive(Clone, Debug)]
pub struct NonNewtonianModel {
    /// Model type
    pub model_type: NonNewtonianType,
    /// Zero-shear viscosity μ₀ (Pa·s)
    pub viscosity_0: f32,
    /// Infinite-shear viscosity μ∞ (Pa·s)
    pub viscosity_inf: f32,
    /// Power-law index n (< 1 = thinning, > 1 = thickening)
    pub power_index: f32,
    /// Relaxation time λ (seconds)
    pub relaxation_time: f32,
    /// Yield stress τ_y (Pa) for Bingham
    pub yield_stress: f32,
    /// Cross model exponent m
    pub cross_exponent: f32,
}

impl Default for NonNewtonianModel {
    fn default() -> Self {
        Self {
            model_type: NonNewtonianType::Newtonian,
            viscosity_0: 0.001,
            viscosity_inf: 0.001,
            power_index: 1.0,
            relaxation_time: 1.0,
            yield_stress: 0.0,
            cross_exponent: 2.0,
        }
    }
}

impl NonNewtonianModel {
    /// Create Carreau model for shear-thinning fluids
    ///
    /// # Arguments
    /// * `mu_0` - Zero-shear viscosity
    /// * `n` - Power index (< 1 for thinning)
    /// * `lambda` - Relaxation time
    /// * `mu_inf` - Infinite-shear viscosity
    pub fn carreau(mu_0: f32, n: f32, lambda: f32, mu_inf: f32) -> Self {
        Self {
            model_type: NonNewtonianType::Carreau,
            viscosity_0: mu_0,
            viscosity_inf: mu_inf,
            power_index: n,
            relaxation_time: lambda,
            ..Default::default()
        }
    }

    /// Create Power Law model
    ///
    /// # Arguments
    /// * `k` - Consistency index
    /// * `n` - Power index
    pub fn power_law(k: f32, n: f32) -> Self {
        Self {
            model_type: NonNewtonianType::PowerLaw,
            viscosity_0: k,
            power_index: n,
            ..Default::default()
        }
    }

    /// Create Cross model for polymer solutions
    pub fn cross(mu_0: f32, mu_inf: f32, lambda: f32, m: f32) -> Self {
        Self {
            model_type: NonNewtonianType::Cross,
            viscosity_0: mu_0,
            viscosity_inf: mu_inf,
            relaxation_time: lambda,
            cross_exponent: m,
            ..Default::default()
        }
    }

    /// Create Bingham plastic model (toothpaste, mayonnaise)
    pub fn bingham(mu_0: f32, yield_stress: f32) -> Self {
        Self {
            model_type: NonNewtonianType::Bingham,
            viscosity_0: mu_0,
            yield_stress,
            ..Default::default()
        }
    }

    /// Compute effective viscosity for given shear rate
    ///
    /// # Arguments
    /// * `shear_rate` - Local shear rate γ̇ (1/s)
    ///
    /// # Returns
    /// Effective dynamic viscosity (Pa·s)
    pub fn compute_viscosity(&self, shear_rate: f32) -> f32 {
        let gamma_dot = shear_rate.max(1e-6); // Prevent division by zero

        match self.model_type {
            NonNewtonianType::Newtonian => self.viscosity_0,

            NonNewtonianType::PowerLaw => {
                // μ = K γ̇^(n-1)
                self.viscosity_0 * gamma_dot.powf(self.power_index - 1.0)
            }

            NonNewtonianType::Carreau => {
                // μ = μ∞ + (μ₀ - μ∞)(1 + (λγ̇)²)^((n-1)/2)
                let lambda_gamma = self.relaxation_time * gamma_dot;
                let factor =
                    (1.0 + lambda_gamma * lambda_gamma).powf((self.power_index - 1.0) / 2.0);
                self.viscosity_inf + (self.viscosity_0 - self.viscosity_inf) * factor
            }

            NonNewtonianType::Cross => {
                // μ = μ∞ + (μ₀ - μ∞)/(1 + (λγ̇)^m)
                let lambda_gamma = self.relaxation_time * gamma_dot;
                let denom = 1.0 + lambda_gamma.powf(self.cross_exponent);
                self.viscosity_inf + (self.viscosity_0 - self.viscosity_inf) / denom
            }

            NonNewtonianType::Bingham => {
                // μ = μ₀ + τ_y/γ̇ (regularized)
                self.viscosity_0 + self.yield_stress / gamma_dot
            }
        }
    }
}

// ============================================================================
// TEMPERATURE MODELS
// ============================================================================

/// Temperature-dependent viscosity model
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[non_exhaustive]
pub enum TemperatureType {
    /// Constant (no temperature dependence)
    #[default]
    Constant,
    /// Arrhenius: μ(T) = A exp(E_a / RT)
    Arrhenius,
    /// VTF (Vogel-Tammann-Fulcher): μ(T) = μ_ref exp(B(1/T - 1/T_ref))
    VTF,
    /// Williams-Landel-Ferry (for polymers near glass transition)
    WLF,
}

/// Parameters for temperature-dependent viscosity
#[derive(Clone, Debug)]
pub struct TemperatureModel {
    /// Model type
    pub model_type: TemperatureType,
    /// Reference viscosity at T_ref (Pa·s)
    pub reference_viscosity: f32,
    /// Reference temperature (Kelvin)
    pub reference_temp: f32,
    /// Activation energy E_a (J/mol) for Arrhenius
    pub activation_energy: f32,
    /// Temperature coefficient B for VTF
    pub temp_coefficient: f32,
    /// VTF reference temperature T₀ (Vogel temperature)
    pub vogel_temp: f32,
    /// WLF constants C1, C2
    pub wlf_c1: f32,
    pub wlf_c2: f32,
}

impl Default for TemperatureModel {
    fn default() -> Self {
        Self {
            model_type: TemperatureType::Constant,
            reference_viscosity: 0.001,
            reference_temp: 293.0,      // 20°C
            activation_energy: 16000.0, // Typical for water
            temp_coefficient: 600.0,
            vogel_temp: 140.0,
            wlf_c1: 17.44,
            wlf_c2: 51.6,
        }
    }
}

impl TemperatureModel {
    /// Create Arrhenius model for liquids
    ///
    /// # Arguments
    /// * `mu_ref` - Viscosity at reference temperature
    /// * `t_ref` - Reference temperature (K)
    /// * `e_a` - Activation energy (J/mol)
    pub fn arrhenius(mu_ref: f32, t_ref: f32, e_a: f32) -> Self {
        Self {
            model_type: TemperatureType::Arrhenius,
            reference_viscosity: mu_ref,
            reference_temp: t_ref,
            activation_energy: e_a,
            ..Default::default()
        }
    }

    /// Create VTF model for glass-forming liquids
    pub fn vtf(mu_ref: f32, t_ref: f32, b: f32, t0: f32) -> Self {
        Self {
            model_type: TemperatureType::VTF,
            reference_viscosity: mu_ref,
            reference_temp: t_ref,
            temp_coefficient: b,
            vogel_temp: t0,
            ..Default::default()
        }
    }

    /// Create model for water
    pub fn water() -> Self {
        Self::arrhenius(0.001, 293.0, 16000.0)
    }

    /// Create model for honey
    pub fn honey() -> Self {
        Self::vtf(5.0, 293.0, 4000.0, 180.0)
    }

    /// Compute viscosity at given temperature
    ///
    /// # Arguments
    /// * `temperature` - Temperature in Kelvin
    ///
    /// # Returns
    /// Dynamic viscosity (Pa·s)
    pub fn compute_viscosity(&self, temperature: f32) -> f32 {
        const R: f32 = 8.314; // Gas constant J/(mol·K)

        match self.model_type {
            TemperatureType::Constant => self.reference_viscosity,

            TemperatureType::Arrhenius => {
                // μ(T) = μ_ref exp(E_a/R (1/T - 1/T_ref))
                let exponent =
                    (self.activation_energy / R) * (1.0 / temperature - 1.0 / self.reference_temp);
                self.reference_viscosity * exponent.exp()
            }

            TemperatureType::VTF => {
                // μ(T) = μ_ref exp(B/(T - T₀) - B/(T_ref - T₀))
                let t_minus_t0 = (temperature - self.vogel_temp).max(1.0);
                let tref_minus_t0 = (self.reference_temp - self.vogel_temp).max(1.0);
                let exponent = self.temp_coefficient * (1.0 / t_minus_t0 - 1.0 / tref_minus_t0);
                self.reference_viscosity * exponent.exp()
            }

            TemperatureType::WLF => {
                // log(μ/μ_ref) = -C1(T - T_ref) / (C2 + T - T_ref)
                let delta_t = temperature - self.reference_temp;
                let log_shift = -self.wlf_c1 * delta_t / (self.wlf_c2 + delta_t);
                self.reference_viscosity * (10.0_f32).powf(log_shift)
            }
        }
    }
}

// ============================================================================
// VISCOSITY SOLVER (CPU REFERENCE IMPLEMENTATION)
// ============================================================================

/// CPU reference implementation of viscosity solver
///
/// This is used for testing and validation. There is currently NO GPU
/// implementation of this module's solvers: earlier doc revisions referenced
/// `viscosity_morris.wgsl` (an orphan shader with no `include_str!` consumer,
/// deleted in F.1) and `viscosity_implicit.wgsl` (which never existed). The
/// experimental `PcisphSystem` has its own Morris viscosity kernel inside
/// `shaders/research/pcisph.wgsl`.
pub struct ViscositySolverCpu {
    config: ViscosityConfig,
    /// Intermediate velocity buffer for implicit solver
    velocity_buffer: Vec<[f32; 3]>,
    /// Per-iteration residual for convergence check
    residuals: Vec<f32>,
}

impl ViscositySolverCpu {
    /// Create new viscosity solver
    pub fn new(config: ViscosityConfig, max_particles: usize) -> Self {
        Self {
            config,
            velocity_buffer: vec![[0.0; 3]; max_particles],
            residuals: Vec::with_capacity(50),
        }
    }

    /// Get current configuration
    pub fn config(&self) -> &ViscosityConfig {
        &self.config
    }

    /// Get solver method
    pub fn solver_method(&self) -> ViscositySolver {
        self.config.solver
    }

    /// Get residual history from last solve
    pub fn residuals(&self) -> &[f32] {
        &self.residuals
    }

    /// Compute effective viscosity for a particle
    ///
    /// Accounts for non-Newtonian and temperature effects if enabled.
    pub fn get_effective_viscosity(&self, particle: &ResearchParticle) -> f32 {
        let mut mu = self.config.base_viscosity;

        // Apply temperature dependence
        if self.config.enable_temperature {
            mu = self
                .config
                .temperature_model
                .compute_viscosity(particle.temperature);
        }

        // Apply non-Newtonian model
        if self.config.enable_non_newtonian {
            let shear_rate = particle.shear_rate.max(1e-6);
            // Scale the non-Newtonian model using base viscosity ratio
            let nn_mu = self
                .config
                .non_newtonian_model
                .compute_viscosity(shear_rate);
            mu *= nn_mu / self.config.non_newtonian_model.viscosity_0;
        }

        mu
    }

    /// Apply viscosity forces using Morris formulation (explicit)
    ///
    /// Morris et al. 1997: "Modeling Low Reynolds Number Incompressible Flows"
    ///
    /// The full formula with dimension-dependent Laplacian approximation:
    /// a_i = 2(d+2) Σⱼ mⱼ (μᵢ + μⱼ)/(ρᵢρⱼ) (vⱼ - vᵢ)·rᵢⱼ/(|rᵢⱼ|² + εh²) ∇Wᵢⱼ
    ///
    /// For 3D: 2*(3+2) = 10, but since we use (μ_i + μ_j) instead of 2μ, factor is 5
    pub fn apply_morris_viscosity(&self, particles: &mut [ResearchParticle]) {
        let h = self.config.smoothing_radius;
        let dt = self.config.dt;
        let h_sq = h * h;
        let epsilon = 0.01 * h_sq;

        // Dimension factor for 3D Laplacian approximation: 2*(d+2)/2 = d+2 = 5
        // (divided by 2 because we use (μ_i + μ_j) instead of 2μ)
        let dim_factor = 5.0_f32;

        // Collect positions and velocities to avoid borrow issues
        let positions: Vec<[f32; 3]> = particles
            .iter()
            .map(|p| [p.position[0], p.position[1], p.position[2]])
            .collect();
        let velocities: Vec<[f32; 3]> = particles
            .iter()
            .map(|p| [p.velocity[0], p.velocity[1], p.velocity[2]])
            .collect();
        let densities: Vec<f32> = particles.iter().map(|p| p.density).collect();
        let viscosities: Vec<f32> = particles
            .iter()
            .map(|p| self.get_effective_viscosity(p))
            .collect();

        // Compute viscous accelerations
        let mut accels = vec![[0.0f32; 3]; particles.len()];

        for i in 0..particles.len() {
            let pos_i = positions[i];
            let vel_i = velocities[i];
            let rho_i = densities[i];
            let mu_i = viscosities[i];

            let mut accel = [0.0f32; 3];

            for j in 0..particles.len() {
                if i == j {
                    continue;
                }

                let pos_j = positions[j];
                let diff = [
                    pos_i[0] - pos_j[0],
                    pos_i[1] - pos_j[1],
                    pos_i[2] - pos_j[2],
                ];
                let r_sq = diff[0] * diff[0] + diff[1] * diff[1] + diff[2] * diff[2];
                let r = r_sq.sqrt();

                if r >= h || r < 1e-10 {
                    continue;
                }

                let vel_j = velocities[j];
                let rho_j = densities[j];
                let mu_j = viscosities[j];
                let m_j = self.config.particle_mass;

                // Morris formulation (Morris et al. 1997) with 3D dimension factor
                // a_i = dim_factor * m_j * (μ_i + μ_j)/(ρ_i * ρ_j) * (v_j - v_i) * |∇W| * r / (r² + εh²)
                //
                // The formula produces a diffusive effect: particles with higher velocity
                // than neighbors are slowed down, particles with lower velocity are sped up.
                let mu_avg = (mu_i + mu_j) / (rho_i * rho_j);
                let grad_w_mag = kernel_gradient_mag(r, h);

                // Include dimension factor for proper Laplacian approximation
                let factor = dim_factor * m_j * mu_avg * grad_w_mag * r / (r_sq + epsilon);

                // (v_j - v_i) means if we're faster, we get negative acceleration (slow down)
                accel[0] += factor * (vel_j[0] - vel_i[0]);
                accel[1] += factor * (vel_j[1] - vel_i[1]);
                accel[2] += factor * (vel_j[2] - vel_i[2]);
            }

            accels[i] = accel;
        }

        // Apply accelerations
        for (i, particle) in particles.iter_mut().enumerate() {
            particle.velocity[0] += accels[i][0] * dt;
            particle.velocity[1] += accels[i][1] * dt;
            particle.velocity[2] += accels[i][2] * dt;
        }
    }

    /// Apply viscosity using matrix-free implicit Jacobi iteration
    ///
    /// Weiler et al. 2018: GPU-efficient implicit viscosity
    ///
    /// Solves: (I - dt μ/ρ ∇²) v^(n+1) = v^n
    pub fn apply_implicit_viscosity(&mut self, particles: &mut [ResearchParticle]) {
        let h = self.config.smoothing_radius;
        let dt = self.config.dt;
        let omega = self.config.omega;

        self.residuals.clear();

        // Collect particle data
        let positions: Vec<[f32; 3]> = particles
            .iter()
            .map(|p| [p.position[0], p.position[1], p.position[2]])
            .collect();
        let densities: Vec<f32> = particles.iter().map(|p| p.density).collect();
        let viscosities: Vec<f32> = particles
            .iter()
            .map(|p| self.get_effective_viscosity(p))
            .collect();

        // Initialize velocity buffer with current velocities
        for (i, p) in particles.iter().enumerate() {
            self.velocity_buffer[i] = [p.velocity[0], p.velocity[1], p.velocity[2]];
        }

        // Store original velocities
        let v_original: Vec<[f32; 3]> = self.velocity_buffer[..particles.len()].to_vec();

        // Jacobi iterations
        for _iter in 0..self.config.max_iterations {
            let mut max_residual = 0.0f32;
            let v_old = self.velocity_buffer[..particles.len()].to_vec();

            for i in 0..particles.len() {
                let pos_i = positions[i];
                let _rho_i = densities[i];
                let mu_i = viscosities[i];

                let mut weighted_sum = [0.0f32; 3];
                let mut weight_total = 0.0f32;

                for j in 0..particles.len() {
                    if i == j {
                        continue;
                    }

                    let pos_j = positions[j];
                    let diff = [
                        pos_i[0] - pos_j[0],
                        pos_i[1] - pos_j[1],
                        pos_i[2] - pos_j[2],
                    ];
                    let r = (diff[0] * diff[0] + diff[1] * diff[1] + diff[2] * diff[2]).sqrt();

                    if r >= h {
                        continue;
                    }

                    let rho_j = densities[j];
                    let mu_j = viscosities[j];
                    let m_j = self.config.particle_mass;

                    // Harmonic mean of viscosities
                    let mu_avg = 2.0 * mu_i * mu_j / (mu_i + mu_j + 1e-10);

                    // Laplacian kernel
                    let lap_w = kernel_laplacian(r, h);

                    // Weight for this neighbor
                    let weight = dt * mu_avg * lap_w * m_j / rho_j;

                    weighted_sum[0] += weight * v_old[j][0];
                    weighted_sum[1] += weight * v_old[j][1];
                    weighted_sum[2] += weight * v_old[j][2];
                    weight_total += weight;
                }

                // Jacobi update with SOR
                let denom = 1.0 + weight_total;
                let new_v = [
                    (v_original[i][0] + weighted_sum[0]) / denom,
                    (v_original[i][1] + weighted_sum[1]) / denom,
                    (v_original[i][2] + weighted_sum[2]) / denom,
                ];

                // Apply SOR relaxation
                self.velocity_buffer[i] = [
                    v_old[i][0] + omega * (new_v[0] - v_old[i][0]),
                    v_old[i][1] + omega * (new_v[1] - v_old[i][1]),
                    v_old[i][2] + omega * (new_v[2] - v_old[i][2]),
                ];

                // Track residual
                let residual = ((new_v[0] - v_old[i][0]).powi(2)
                    + (new_v[1] - v_old[i][1]).powi(2)
                    + (new_v[2] - v_old[i][2]).powi(2))
                .sqrt();
                max_residual = max_residual.max(residual);
            }

            self.residuals.push(max_residual);

            // Check convergence
            if max_residual < self.config.tolerance {
                break;
            }
        }

        // Apply final velocities
        for (i, particle) in particles.iter_mut().enumerate() {
            particle.velocity[0] = self.velocity_buffer[i][0];
            particle.velocity[1] = self.velocity_buffer[i][1];
            particle.velocity[2] = self.velocity_buffer[i][2];
        }
    }

    /// Apply XSPH viscosity (simplified, for games)
    pub fn apply_xsph_viscosity(&self, particles: &mut [ResearchParticle]) {
        let h = self.config.smoothing_radius;
        let xsph_factor = 0.01; // Fixed XSPH coefficient

        // Collect data
        let positions: Vec<[f32; 3]> = particles
            .iter()
            .map(|p| [p.position[0], p.position[1], p.position[2]])
            .collect();
        let velocities: Vec<[f32; 3]> = particles
            .iter()
            .map(|p| [p.velocity[0], p.velocity[1], p.velocity[2]])
            .collect();

        let mut corrections = vec![[0.0f32; 3]; particles.len()];

        for i in 0..particles.len() {
            let pos_i = positions[i];
            let vel_i = velocities[i];

            let mut correction = [0.0f32; 3];

            for j in 0..particles.len() {
                if i == j {
                    continue;
                }

                let pos_j = positions[j];
                let diff = [
                    pos_i[0] - pos_j[0],
                    pos_i[1] - pos_j[1],
                    pos_i[2] - pos_j[2],
                ];
                let r = (diff[0] * diff[0] + diff[1] * diff[1] + diff[2] * diff[2]).sqrt();

                if r >= h {
                    continue;
                }

                let vel_j = velocities[j];
                let w = kernel_w(r, h);

                correction[0] += xsph_factor * (vel_j[0] - vel_i[0]) * w;
                correction[1] += xsph_factor * (vel_j[1] - vel_i[1]) * w;
                correction[2] += xsph_factor * (vel_j[2] - vel_i[2]) * w;
            }

            corrections[i] = correction;
        }

        // Apply corrections
        for (i, particle) in particles.iter_mut().enumerate() {
            particle.velocity[0] += corrections[i][0];
            particle.velocity[1] += corrections[i][1];
            particle.velocity[2] += corrections[i][2];
        }
    }

    /// Apply viscosity using the configured solver method
    pub fn apply(&mut self, particles: &mut [ResearchParticle]) {
        match self.config.solver {
            ViscositySolver::XSPH => self.apply_xsph_viscosity(particles),
            ViscositySolver::Morris => self.apply_morris_viscosity(particles),
            ViscositySolver::ImplicitJacobi => self.apply_implicit_viscosity(particles),
        }
    }
}

// ============================================================================
// SHEAR RATE COMPUTATION
// ============================================================================

/// Compute shear rate for a particle using the configured method
pub fn compute_shear_rate(
    particle_idx: usize,
    particles: &[ResearchParticle],
    method: ShearRateMethod,
    h: f32,
) -> f32 {
    match method {
        ShearRateMethod::VorticityBased => compute_shear_rate_vorticity(particle_idx, particles),
        ShearRateMethod::StrainTensor => compute_shear_rate_strain(particle_idx, particles, h),
        ShearRateMethod::Blended => {
            let vort = compute_shear_rate_vorticity(particle_idx, particles);
            let strain = compute_shear_rate_strain(particle_idx, particles, h);
            0.7 * vort + 0.3 * strain
        }
    }
}

/// Compute shear rate from vorticity magnitude (smoother)
fn compute_shear_rate_vorticity(particle_idx: usize, particles: &[ResearchParticle]) -> f32 {
    let omega = particles[particle_idx].vorticity;
    (omega[0] * omega[0] + omega[1] * omega[1] + omega[2] * omega[2]).sqrt()
}

/// Compute shear rate from strain tensor (more accurate but noisy)
#[allow(clippy::needless_range_loop)]
fn compute_shear_rate_strain(particle_idx: usize, particles: &[ResearchParticle], h: f32) -> f32 {
    let pos_i = [
        particles[particle_idx].position[0],
        particles[particle_idx].position[1],
        particles[particle_idx].position[2],
    ];
    let vel_i = [
        particles[particle_idx].velocity[0],
        particles[particle_idx].velocity[1],
        particles[particle_idx].velocity[2],
    ];

    // Compute velocity gradient tensor
    let mut grad_v = [[0.0f32; 3]; 3]; // ∂vᵢ/∂xⱼ

    for (j, p_j) in particles.iter().enumerate() {
        if j == particle_idx {
            continue;
        }

        let pos_j = [p_j.position[0], p_j.position[1], p_j.position[2]];
        let vel_j = [p_j.velocity[0], p_j.velocity[1], p_j.velocity[2]];

        let diff = [
            pos_i[0] - pos_j[0],
            pos_i[1] - pos_j[1],
            pos_i[2] - pos_j[2],
        ];
        let r = (diff[0] * diff[0] + diff[1] * diff[1] + diff[2] * diff[2]).sqrt();

        if r >= h || r < 1e-10 {
            continue;
        }

        let grad_w = kernel_gradient_mag(r, h);
        let grad_dir = [-diff[0] / r, -diff[1] / r, -diff[2] / r];

        let vel_diff = [
            vel_j[0] - vel_i[0],
            vel_j[1] - vel_i[1],
            vel_j[2] - vel_i[2],
        ];

        // Outer product of velocity difference and kernel gradient
        for a in 0..3 {
            for b in 0..3 {
                grad_v[a][b] += vel_diff[a] * grad_dir[b] * grad_w;
            }
        }
    }

    // Strain rate tensor: D = 0.5 (∇v + ∇vᵀ)
    let mut d = [[0.0f32; 3]; 3];
    for a in 0..3 {
        for b in 0..3 {
            d[a][b] = 0.5 * (grad_v[a][b] + grad_v[b][a]);
        }
    }

    // Shear rate = sqrt(2 * D:D) = sqrt(2 * Σᵢⱼ Dᵢⱼ²)
    let mut d_sq_sum = 0.0f32;
    for a in 0..3 {
        for b in 0..3 {
            d_sq_sum += d[a][b] * d[a][b];
        }
    }

    (2.0 * d_sq_sum).sqrt()
}

// ============================================================================
// STATISTICS
// ============================================================================

/// Statistics from viscosity computation
#[derive(Clone, Debug, Default)]
pub struct ViscosityStats {
    /// Minimum effective viscosity (Pa·s)
    pub min_viscosity: f32,
    /// Maximum effective viscosity (Pa·s)
    pub max_viscosity: f32,
    /// Average effective viscosity (Pa·s)
    pub avg_viscosity: f32,
    /// Maximum shear rate (1/s)
    pub max_shear_rate: f32,
    /// Number of Jacobi iterations (if implicit)
    pub iterations: u32,
    /// Final residual (if implicit)
    pub final_residual: f32,
}

impl ViscosityStats {
    /// Compute statistics from particles and solver
    pub fn compute(particles: &[ResearchParticle], solver: &ViscositySolverCpu) -> Self {
        if particles.is_empty() {
            return Self::default();
        }

        let mut min_mu = f32::MAX;
        let mut max_mu = 0.0f32;
        let mut sum_mu = 0.0f32;
        let mut max_shear = 0.0f32;

        for p in particles {
            let mu = solver.get_effective_viscosity(p);
            min_mu = min_mu.min(mu);
            max_mu = max_mu.max(mu);
            sum_mu += mu;
            max_shear = max_shear.max(p.shear_rate);
        }

        let residuals = solver.residuals();

        Self {
            min_viscosity: min_mu,
            max_viscosity: max_mu,
            avg_viscosity: sum_mu / particles.len() as f32,
            max_shear_rate: max_shear,
            iterations: residuals.len() as u32,
            final_residual: residuals.last().copied().unwrap_or(0.0),
        }
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kernel_w_normalization() {
        // Kernel should integrate to approximately 1
        let h = 1.0;
        let dr = 0.01;
        let mut integral = 0.0f32;

        let mut r = 0.0;
        while r < h {
            // 4πr² dr for spherical integration
            integral += kernel_w(r, h) * 4.0 * std::f32::consts::PI * r * r * dr;
            r += dr;
        }

        // Should be close to 1 (within numerical tolerance)
        assert!(
            (integral - 1.0).abs() < 0.1,
            "Kernel integral: {}",
            integral
        );
    }

    #[test]
    fn test_kernel_gradient_at_zero() {
        // Gradient should be zero at r=0
        let grad = kernel_gradient_mag(0.0, 1.0);
        assert_eq!(grad, 0.0);
    }

    #[test]
    fn test_kernel_outside_support() {
        let h = 1.0;
        assert_eq!(kernel_w(1.5, h), 0.0);
        assert_eq!(kernel_gradient_mag(1.5, h), 0.0);
        assert_eq!(kernel_laplacian(1.5, h), 0.0);
    }

    #[test]
    fn test_viscosity_config_presets() {
        let water = ViscosityConfig::water();
        assert!((water.base_viscosity - 0.001).abs() < 1e-6);

        let honey = ViscosityConfig::honey();
        assert!(honey.base_viscosity > 1.0);
        assert!(matches!(honey.solver, ViscositySolver::ImplicitJacobi));

        let thinning = ViscosityConfig::shear_thinning();
        assert!(thinning.enable_non_newtonian);
    }

    #[test]
    fn test_carreau_model() {
        // Shear-thinning: viscosity decreases with shear rate
        let model = NonNewtonianModel::carreau(1.0, 0.5, 1.0, 0.01);

        let mu_low = model.compute_viscosity(0.01);
        let mu_high = model.compute_viscosity(100.0);

        assert!(
            mu_low > mu_high,
            "Shear-thinning: low shear should have higher viscosity"
        );
    }

    #[test]
    fn test_power_law_shear_thickening() {
        // n > 1 means shear-thickening
        let model = NonNewtonianModel::power_law(0.1, 1.5);

        let mu_low = model.compute_viscosity(0.1);
        let mu_high = model.compute_viscosity(10.0);

        assert!(
            mu_high > mu_low,
            "Shear-thickening: high shear should have higher viscosity"
        );
    }

    #[test]
    fn test_bingham_yield_stress() {
        let model = NonNewtonianModel::bingham(0.1, 10.0);

        // At very low shear, effective viscosity is very high (resists flow)
        let mu_low = model.compute_viscosity(0.01);
        let mu_high = model.compute_viscosity(100.0);

        assert!(
            mu_low > mu_high * 10.0,
            "Bingham: low shear should have much higher viscosity"
        );
    }

    #[test]
    fn test_arrhenius_temperature() {
        let model = TemperatureModel::arrhenius(0.001, 293.0, 16000.0);

        // Viscosity should decrease with temperature
        let mu_cold = model.compute_viscosity(273.0); // 0°C
        let mu_hot = model.compute_viscosity(373.0); // 100°C

        assert!(
            mu_cold > mu_hot,
            "Viscosity should decrease with temperature"
        );
    }

    #[test]
    fn test_vtf_model() {
        let model = TemperatureModel::vtf(5.0, 293.0, 600.0, 140.0);

        let mu_cold = model.compute_viscosity(250.0);
        let mu_hot = model.compute_viscosity(350.0);

        assert!(
            mu_cold > mu_hot,
            "VTF: viscosity should decrease with temperature"
        );
    }

    #[test]
    fn test_viscosity_solver_creation() {
        let config = ViscosityConfig::default();
        let solver = ViscositySolverCpu::new(config, 1000);

        assert!(matches!(solver.solver_method(), ViscositySolver::Morris));
    }

    #[test]
    fn test_effective_viscosity_newtonian() {
        let config = ViscosityConfig {
            base_viscosity: 0.5,
            enable_non_newtonian: false,
            enable_temperature: false,
            ..Default::default()
        };
        let solver = ViscositySolverCpu::new(config, 100);

        let particle = ResearchParticle::default();
        let mu = solver.get_effective_viscosity(&particle);

        assert!((mu - 0.5).abs() < 1e-6);
    }

    #[test]
    fn test_xsph_viscosity_smoothing() {
        // Create a simple 2-particle system
        let config = ViscosityConfig {
            solver: ViscositySolver::XSPH,
            smoothing_radius: 2.0,
            ..Default::default()
        };
        let solver = ViscositySolverCpu::new(config, 100);

        let mut particles = vec![
            ResearchParticle {
                position: [0.0, 0.0, 0.0, 1.0],
                velocity: [1.0, 0.0, 0.0, 0.0],
                density: 1000.0,
                ..Default::default()
            },
            ResearchParticle {
                position: [0.5, 0.0, 0.0, 1.0],
                velocity: [-1.0, 0.0, 0.0, 0.0],
                density: 1000.0,
                ..Default::default()
            },
        ];

        solver.apply_xsph_viscosity(&mut particles);

        // Velocities should move towards each other
        assert!(
            particles[0].velocity[0] < 1.0,
            "First particle should slow down"
        );
        assert!(
            particles[1].velocity[0] > -1.0,
            "Second particle should slow down"
        );
    }

    #[test]
    fn test_morris_viscosity_effect() {
        // Use very high viscosity to see noticeable effect in single step
        // (real simulations would run thousands of steps)
        let config = ViscosityConfig {
            solver: ViscositySolver::Morris,
            base_viscosity: 1000.0, // Very high viscosity (like thick honey * 100)
            smoothing_radius: 2.0,
            dt: 0.1, // Larger timestep
            particle_mass: 1.0,
            ..Default::default()
        };
        let solver = ViscositySolverCpu::new(config, 100);

        let mut particles = vec![
            ResearchParticle {
                position: [0.0, 0.0, 0.0, 1.0],
                velocity: [1.0, 0.0, 0.0, 0.0],
                density: 1000.0,
                ..Default::default()
            },
            ResearchParticle {
                position: [0.5, 0.0, 0.0, 1.0],
                velocity: [0.0, 0.0, 0.0, 0.0],
                density: 1000.0,
                ..Default::default()
            },
        ];

        let v0_before = particles[0].velocity[0];
        let v1_before = particles[1].velocity[0];

        solver.apply_morris_viscosity(&mut particles);

        // First particle should slow down due to viscous drag
        // Second particle should speed up (momentum conservation)
        assert!(
            particles[0].velocity[0] < v0_before,
            "Particle 0 should experience viscous drag: {} -> {}",
            v0_before,
            particles[0].velocity[0]
        );
        assert!(
            particles[1].velocity[0] > v1_before,
            "Particle 1 should speed up: {} -> {}",
            v1_before,
            particles[1].velocity[0]
        );
    }

    #[test]
    fn test_implicit_solver_convergence() {
        let config = ViscosityConfig {
            solver: ViscositySolver::ImplicitJacobi,
            base_viscosity: 1.0,
            smoothing_radius: 2.0,
            dt: 0.01,
            particle_mass: 1.0,
            max_iterations: 20,
            tolerance: 1e-4,
            ..Default::default()
        };
        let mut solver = ViscositySolverCpu::new(config, 100);

        let mut particles = vec![
            ResearchParticle {
                position: [0.0, 0.0, 0.0, 1.0],
                velocity: [1.0, 0.0, 0.0, 0.0],
                density: 1000.0,
                ..Default::default()
            },
            ResearchParticle {
                position: [0.5, 0.0, 0.0, 1.0],
                velocity: [0.0, 0.0, 0.0, 0.0],
                density: 1000.0,
                ..Default::default()
            },
        ];

        solver.apply_implicit_viscosity(&mut particles);

        // Check that solver converged
        let residuals = solver.residuals();
        assert!(!residuals.is_empty(), "Should have residual history");

        // Final residual should be small
        if residuals.len() > 1 {
            assert!(
                residuals.last().unwrap() < residuals.first().unwrap(),
                "Residuals should decrease"
            );
        }
    }

    #[test]
    fn test_shear_rate_vorticity() {
        let particles = vec![ResearchParticle {
            vorticity: [1.0, 2.0, 2.0], // magnitude = 3
            ..Default::default()
        }];

        let shear = compute_shear_rate(0, &particles, ShearRateMethod::VorticityBased, 1.0);
        assert!((shear - 3.0).abs() < 1e-6);
    }

    #[test]
    fn test_stats_computation() {
        let config = ViscosityConfig::water();
        let solver = ViscositySolverCpu::new(config, 100);

        let particles = vec![
            ResearchParticle {
                density: 1000.0,
                shear_rate: 10.0,
                ..Default::default()
            },
            ResearchParticle {
                density: 1000.0,
                shear_rate: 50.0,
                ..Default::default()
            },
        ];

        let stats = ViscosityStats::compute(&particles, &solver);

        assert!((stats.min_viscosity - 0.001).abs() < 1e-6);
        assert!((stats.max_viscosity - 0.001).abs() < 1e-6);
        assert!((stats.max_shear_rate - 50.0).abs() < 1e-6);
    }

    #[test]
    fn test_cross_model() {
        let model = NonNewtonianModel::cross(1.0, 0.01, 1.0, 2.0);

        let mu_low = model.compute_viscosity(0.01);
        let mu_high = model.compute_viscosity(100.0);

        // Cross model is shear-thinning
        assert!(mu_low > mu_high);
        assert!(mu_low <= 1.0); // Should not exceed mu_0
        assert!(mu_high >= 0.01); // Should not go below mu_inf
    }
}
