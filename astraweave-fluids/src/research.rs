//! # Research-Grade SPH Fluid Simulation
//!
//! This module provides research-grade fluid simulation capabilities, implementing:
//!
//! ## Solvers
//! - **PCISPH**: Predictive-Corrective Incompressible SPH (Solenthaler & Pajarola 2009)
//! - (DFSPH / IISPH variants removed in F.1 - no implementation existed;
//!   CPU building blocks remain in simd_ops for a future campaign)
//!
//! ## Stability Features
//! - **δ-SPH Particle Shifting**: Fixes tensile instability (Marrone et al. 2011)
//! - **δ⁺-SPH Multi-Phase**: Interface-aware shifting (Sun et al. 2017)
//! - **Warm-Starting**: Reuse previous pressure for faster convergence
//!
//! ## Viscosity Models
//! - **Morris**: Physically-based explicit viscosity (Morris et al. 1997)
//! - **Implicit Jacobi**: Matrix-free implicit solver (Weiler et al. 2018)
//! - **Non-Newtonian**: Carreau model for shear-thinning/thickening
//!
//! ## Turbulence
//! - **Vorticity Confinement**: Re-inject lost small-scale vortices
//! - **Micropolar SPH**: Particle spin for fine turbulence (optional)
//!
//! ## Usage
//!
//! This module provides the *type family* (configs, GPU layouts, quality
//! tiers) consumed by the experimental `PcisphSystem` solver
//! (`pcisph_system` module, feature `experimental`). There is no
//! `ResearchFluidSystem` type — earlier doc revisions referenced one that
//! was never implemented (corrected in Fluids-Integration F.1).
//!
//! ```ignore
//! use astraweave_fluids::research::{ResearchFluidConfig, SolverType};
//!
//! let config = ResearchFluidConfig {
//!     solver: SolverType::PCISPH,
//!     enable_particle_shifting: true,
//!     enable_warm_start: true,
//!     ..Default::default()
//! };
//! // Consumed by pcisph_system::PcisphSystem::new (feature = "experimental").
//! ```

// ============================================================================
// SOLVER TYPE ENUMS
// ============================================================================

/// Incompressibility solver selection
///
/// Only solvers with an actual implementation in this crate are listed.
/// `DFSPH` and `IISPH` variants were removed in Fluids-Integration F.1: no
/// solver loop existed for either (the variants were aspirational), and per
/// the engine's anti-vapor policy enum variants must not promise
/// capabilities that don't exist. CPU building blocks for DFSPH/IISPH remain
/// in `simd_ops` (tested, dormant) should a future campaign implement them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum SolverType {
    /// Position-Based Dynamics - Fast, visual (games)
    #[default]
    PBD,
    /// Predictive-Corrective Incompressible SPH - Balanced
    PCISPH,
}

impl SolverType {
    /// Returns the typical iteration count for this solver
    #[inline]
    pub const fn typical_iterations(self) -> u32 {
        match self {
            Self::PBD => 4,
            Self::PCISPH => 5,
        }
    }

    /// Returns whether this solver benefits from warm-starting
    #[inline]
    pub const fn supports_warm_start(self) -> bool {
        matches!(self, Self::PCISPH)
    }
}

/// Viscosity solver selection
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum ViscositySolver {
    /// XSPH - Fast, artificial (games)
    #[default]
    XSPH,
    /// Morris formulation - Physically-based, explicit
    Morris,
    /// Matrix-free implicit Jacobi - GPU-efficient, high viscosity stable
    ImplicitJacobi,
}

impl ViscositySolver {
    /// Returns whether this solver can handle high viscosity (>1.0 Pa·s)
    #[inline]
    pub const fn supports_high_viscosity(self) -> bool {
        matches!(self, Self::ImplicitJacobi)
    }
}

/// Particle shifting method for tensile instability
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum ShiftingMethod {
    /// No particle shifting
    #[default]
    None,
    /// Standard δ-SPH (single phase, Marrone et al. 2011)
    StandardDelta,
    /// Interface-aware δ⁺-SPH (multi-phase, Sun et al. 2017)
    InterfaceAware,
}

/// Shear rate estimation method for non-Newtonian fluids
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum ShearRateMethod {
    /// Strain tensor - Accurate but noisy
    StrainTensor,
    /// Vorticity-based - Smoother (recommended)
    #[default]
    VorticityBased,
    /// Blended - 70% vorticity + 30% strain
    Blended,
}

/// SPH Kernel function selection
///
/// Different kernels offer different trade-offs:
/// - **Cubic Spline**: Classic SPH kernel, well-understood but prone to pairing instability
/// - **Wendland C2**: Smoother, more stable, recommended for modern SPH (Dehnen & Aly 2012)
/// - **Wendland C4**: Higher smoothness, better for viscous flows
/// - **Wendland C6**: Highest smoothness, research applications
///
/// # References
/// - Wendland (1995) "Piecewise polynomial, positive definite and compactly supported radial functions"
/// - Dehnen & Aly (2012) "Improving convergence in SPH simulations without pairing instability"
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum KernelType {
    /// Classic cubic spline kernel (legacy compatibility)
    CubicSpline,
    /// Wendland C2 - Recommended for most applications
    #[default]
    WendlandC2,
    /// Wendland C4 - Higher smoothness, better for viscous flows
    WendlandC4,
    /// Wendland C6 - Highest smoothness (research)
    WendlandC6,
}

impl KernelType {
    /// Returns the C^n continuity of this kernel
    #[inline]
    pub const fn continuity(self) -> u32 {
        match self {
            Self::CubicSpline => 2,
            Self::WendlandC2 => 2,
            Self::WendlandC4 => 4,
            Self::WendlandC6 => 6,
        }
    }

    /// Returns whether this kernel is recommended for stability
    #[inline]
    pub const fn is_stable(self) -> bool {
        !matches!(self, Self::CubicSpline)
    }

    /// Returns the relative computational cost (1.0 = baseline)
    #[inline]
    pub const fn relative_cost(self) -> f32 {
        match self {
            Self::CubicSpline => 1.0,
            Self::WendlandC2 => 1.1,
            Self::WendlandC4 => 1.3,
            Self::WendlandC6 => 1.5,
        }
    }
}

/// Boundary handling method
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum BoundaryMethod {
    /// Traditional Akinci particle sampling
    AkinciOnly,
    /// SDF-based density contribution (fast, less accurate friction)
    SDFOnly,
    /// Hybrid: SDF for density, Akinci for friction (recommended)
    #[default]
    Hybrid,
}

/// Quality tier for performance scaling
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum ResearchQualityTier {
    /// 50-100k particles, 60 FPS, PBD + XSPH
    Low,
    /// 100-200k particles, 60 FPS, PCISPH + Morris
    #[default]
    Medium,
    /// 200-350k particles, 60 FPS, PCISPH + δ-SPH + vorticity (was DFSPH pre-F.1)
    High,
    /// 350-500k particles, 30 FPS, full features
    Ultra,
    /// 500k-1M particles, offline, all features + VTK export
    Research,
}

// ============================================================================
// RESEARCH PARTICLE STRUCTURE (176 bytes)
// ============================================================================

/// Extended particle structure for research-grade simulation.
///
/// This structure contains all fields needed for DFSPH, viscosity, particle shifting,
/// vorticity, and multi-phase simulation. For games, use the compact 80-byte
/// `Particle` struct from the main module.
///
/// # Memory Layout
/// - Total size: 176 bytes (aligned to 16 bytes)
/// - Designed for GPU buffer storage
///
/// # Performance Note
/// At 1GB VRAM, this structure supports ~6.1M particles (vs 13.4M with 80-byte PBD).
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ResearchParticle {
    // ========== Position-Based (existing, 48 bytes) ==========
    /// World-space position (xyz) + mass (w)
    pub position: [f32; 4],
    /// Current velocity (xyz) + padding (w)
    pub velocity: [f32; 4],
    /// Predicted position for constraint solving (xyz) + padding (w)
    pub predicted_position: [f32; 4],

    // ========== Core SPH (existing, 16 bytes) ==========
    /// Lagrange multiplier for constraint solving (PBD)
    pub lambda: f32,
    /// Current particle density (kg/m³)
    pub density: f32,
    /// Phase identifier (0=water, 1=oil, 2=air, etc.)
    pub phase: u32,
    /// Temperature in Kelvin (ambient ~293K)
    pub temperature: f32,

    // ========== DFSPH/PCISPH (new, 20 bytes) ==========
    /// α factor for density error correction (DFSPH)
    pub alpha: f32,
    /// κ factor for velocity divergence correction (DFSPH)
    pub kappa: f32,
    /// Velocity divergence ∇·v (DFSPH)
    pub velocity_divergence: f32,
    /// Density derivative Dρ/Dt (DFSPH)
    pub density_derivative: f32,
    /// Previous frame pressure for warm-starting
    pub previous_pressure: f32,

    // ========== Viscosity (new, 8 bytes) ==========
    /// Dynamic viscosity coefficient (Pa·s)
    pub viscosity_coefficient: f32,
    /// Local shear rate for non-Newtonian fluids (1/s)
    pub shear_rate: f32,

    // ========== Particle Shifting - δ-SPH (new, 16 bytes) ==========
    /// Shift vector δr for position correction
    pub shift_delta: [f32; 3],
    /// Surface particle flag (0=interior, 1=surface)
    pub is_surface: u32,

    // ========== Vorticity & Turbulence (new, 24 bytes) ==========
    /// Curl of velocity field ω = ∇ × v
    pub vorticity: [f32; 3],
    /// Angular velocity for micropolar SPH
    pub angular_velocity: [f32; 3],

    // ========== Multi-Phase (new, 16 bytes) ==========
    /// Interface normal for δ⁺-SPH (phase boundary detection)
    pub phase_gradient: [f32; 3],
    /// Gas phase flag for air bubbles (0=liquid, 1=gas)
    pub is_gas: u32,

    // ========== Visualization (existing, 16 bytes) ==========
    /// RGBA color for rendering
    pub color: [f32; 4],

    // ========== Reserved for Future / Alignment (12 bytes for 16-byte alignment) ==========
    /// Reserved for future use (e.g., additional solver data)
    pub reserved0: f32,
    /// Reserved for future use
    pub reserved1: f32,
    /// Padding for 16-byte alignment (164 + 12 = 176)
    pub _pad: f32,
}

impl Default for ResearchParticle {
    fn default() -> Self {
        Self {
            position: [0.0, 0.0, 0.0, 1.0],
            velocity: [0.0; 4],
            predicted_position: [0.0, 0.0, 0.0, 1.0],
            lambda: 0.0,
            density: 1000.0, // Water density
            phase: 0,
            temperature: 293.0, // Room temperature
            alpha: 0.0,
            kappa: 0.0,
            velocity_divergence: 0.0,
            density_derivative: 0.0,
            previous_pressure: 0.0,
            viscosity_coefficient: 0.001, // Water viscosity
            shear_rate: 0.0,
            shift_delta: [0.0; 3],
            is_surface: 0,
            vorticity: [0.0; 3],
            angular_velocity: [0.0; 3],
            phase_gradient: [0.0; 3],
            is_gas: 0,
            color: [0.2, 0.5, 0.9, 1.0], // Light blue
            reserved0: 0.0,
            reserved1: 0.0,
            _pad: 0.0,
        }
    }
}

impl ResearchParticle {
    /// Create a new particle at the given position
    #[inline]
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self {
            position: [x, y, z, 1.0],
            predicted_position: [x, y, z, 1.0],
            ..Default::default()
        }
    }

    /// Create a particle with specific phase properties
    pub fn with_phase(x: f32, y: f32, z: f32, phase: u32, density: f32, viscosity: f32) -> Self {
        Self {
            position: [x, y, z, 1.0],
            predicted_position: [x, y, z, 1.0],
            phase,
            density,
            viscosity_coefficient: viscosity,
            ..Default::default()
        }
    }

    /// Check if this particle is a gas (air bubble)
    #[inline]
    pub const fn is_gas_phase(&self) -> bool {
        self.is_gas != 0
    }

    /// Check if this particle is at the free surface
    #[inline]
    pub const fn is_at_surface(&self) -> bool {
        self.is_surface != 0
    }
}

// ============================================================================
// RESEARCH SIMULATION PARAMETERS
// ============================================================================

/// Extended simulation parameters for research-grade SPH
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ResearchSimParams {
    // ========== Core Parameters (64 bytes) ==========
    /// Smoothing radius h (kernel support)
    pub smoothing_radius: f32,
    /// Target rest density ρ₀ (kg/m³)
    pub target_density: f32,
    /// Pressure multiplier for EOS
    pub pressure_multiplier: f32,
    /// Base viscosity coefficient
    pub viscosity: f32,
    /// Surface tension coefficient σ (N/m)
    pub surface_tension: f32,
    /// Gravitational acceleration (m/s²)
    pub gravity: f32,
    /// Time step Δt (seconds)
    pub dt: f32,
    /// Total particle count
    pub particle_count: u32,
    /// Spatial grid width
    pub grid_width: u32,
    /// Spatial grid height
    pub grid_height: u32,
    /// Spatial grid depth
    pub grid_depth: u32,
    /// Grid cell size
    pub cell_size: f32,
    /// Number of dynamic objects
    pub object_count: u32,
    /// Current frame number
    pub frame: u32,
    /// Warm-start factor (0.0-1.0)
    pub warm_start_factor: f32,
    /// Padding
    pub _pad0: f32,

    // ========== Solver Parameters (32 bytes) ==========
    /// Maximum solver iterations
    pub max_iterations: u32,
    /// Minimum solver iterations
    pub min_iterations: u32,
    /// Density error threshold for early exit (0.001 = 0.1%)
    pub density_error_threshold: f32,
    /// Divergence error threshold
    pub divergence_error_threshold: f32,
    /// PCISPH: Pressure coefficient δ
    pub pcisph_delta: f32,
    /// Particle shifting strength C_δ
    pub shifting_strength: f32,
    /// Vorticity confinement strength ε
    pub vorticity_epsilon: f32,
    /// SOR relaxation factor ω (0.5-0.8)
    pub sor_omega: f32,

    // ========== Flags (16 bytes) ==========
    /// Solver type (0=PBD, 1=PCISPH; codes 2/3 reserved — the DFSPH/IISPH
    /// variants were removed in F.1, no implementation existed)
    pub solver_type: u32,
    /// Kernel type (0=CubicSpline, 1=WendlandC2, 2=WendlandC4, 3=WendlandC6)
    pub kernel_type: u32,
    /// Enable particle shifting (0/1)
    pub enable_shifting: u32,
    /// Enable vorticity confinement (0/1)
    pub enable_vorticity: u32,
    /// Enable warm-starting (0/1)
    pub enable_warm_start: u32,
    /// Padding for alignment
    pub _pad1: u32,
    pub _pad2: u32,
    pub _pad3: u32,
}

impl Default for ResearchSimParams {
    fn default() -> Self {
        Self {
            smoothing_radius: 1.2,
            target_density: 1000.0,
            pressure_multiplier: 50.0,
            viscosity: 0.001,
            surface_tension: 0.0728,
            gravity: -9.81,
            dt: 1.0 / 60.0,
            particle_count: 0,
            grid_width: 128,
            grid_height: 64,
            grid_depth: 128,
            cell_size: 1.2,
            object_count: 0,
            frame: 0,
            warm_start_factor: 0.8,
            _pad0: 0.0,

            max_iterations: 20,
            min_iterations: 2,
            density_error_threshold: 0.001,
            divergence_error_threshold: 0.01,
            pcisph_delta: 0.0, // Computed at init
            shifting_strength: 0.04,
            vorticity_epsilon: 0.05,
            sor_omega: 0.7,

            solver_type: 1, // PCISPH
            kernel_type: 1, // WendlandC2 (default for stability)
            enable_shifting: 1,
            enable_vorticity: 1,
            enable_warm_start: 1,
            _pad1: 0,
            _pad2: 0,
            _pad3: 0,
        }
    }
}

// ============================================================================
// RESEARCH FLUID CONFIGURATION
// ============================================================================

/// Configuration for research-grade fluid simulation
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ResearchFluidConfig {
    // ========== Solver Selection ==========
    /// Incompressibility solver type
    pub solver: SolverType,
    /// Viscosity solver type
    pub viscosity_solver: ViscositySolver,
    /// Quality tier
    pub quality_tier: ResearchQualityTier,
    /// SPH kernel function selection (default: Wendland C2)
    pub kernel_type: KernelType,

    // ========== Accuracy Settings ==========
    /// Maximum solver iterations
    pub max_iterations: u32,
    /// Minimum solver iterations
    pub min_iterations: u32,
    /// Target density error (0.001 = 0.1%)
    pub density_error_threshold: f32,

    // ========== Stability Features ==========
    /// Enable δ-SPH particle shifting
    pub enable_particle_shifting: bool,
    /// Particle shifting method
    pub shifting_method: ShiftingMethod,
    /// Enable warm-starting from previous frame
    pub enable_warm_start: bool,
    /// Warm-start blend factor (0.0-1.0)
    pub warm_start_factor: f32,

    // ========== Turbulence ==========
    /// Enable vorticity confinement
    pub enable_vorticity_confinement: bool,
    /// Vorticity confinement strength ε
    pub vorticity_epsilon: f32,
    /// Enable micropolar particle spin
    pub enable_micropolar: bool,

    // ========== Multi-Phase ==========
    /// Enable implicit air phase for bubbles/splashes
    pub enable_implicit_air: bool,
    /// Velocity threshold for splash air spawn (m/s)
    pub air_spawn_threshold: f32,
    /// Maximum air particles
    pub max_air_particles: u32,

    // ========== Viscosity ==========
    /// Enable non-Newtonian viscosity
    pub enable_non_newtonian: bool,
    /// Enable temperature-dependent viscosity
    pub enable_temperature_viscosity: bool,
    /// Shear rate estimation method
    pub shear_rate_method: ShearRateMethod,
    /// Implicit viscosity iterations (for ImplicitJacobi)
    pub viscosity_iterations: u32,

    // ========== Boundaries ==========
    /// Boundary handling method
    pub boundary_method: BoundaryMethod,

    // ========== Validation ==========
    /// Export validation metrics
    pub export_metrics: bool,
    /// Metric export interval (frames)
    pub metric_interval: u32,
    /// Enable VTK export for ParaView
    pub enable_vtk_export: bool,
}

impl Default for ResearchFluidConfig {
    fn default() -> Self {
        Self {
            solver: SolverType::PCISPH,
            viscosity_solver: ViscositySolver::Morris,
            quality_tier: ResearchQualityTier::Medium,
            kernel_type: KernelType::WendlandC2, // Modern default for stability

            max_iterations: 20,
            min_iterations: 2,
            density_error_threshold: 0.001,

            enable_particle_shifting: true,
            shifting_method: ShiftingMethod::StandardDelta,
            enable_warm_start: true,
            warm_start_factor: 0.8,

            enable_vorticity_confinement: true,
            vorticity_epsilon: 0.05,
            enable_micropolar: false,

            enable_implicit_air: false,
            air_spawn_threshold: 5.0,
            max_air_particles: 50000,

            enable_non_newtonian: false,
            enable_temperature_viscosity: false,
            shear_rate_method: ShearRateMethod::VorticityBased,
            viscosity_iterations: 5,

            boundary_method: BoundaryMethod::Hybrid,

            export_metrics: false,
            metric_interval: 60,
            enable_vtk_export: false,
        }
    }
}

impl ResearchFluidConfig {
    /// Create configuration from quality tier preset
    pub fn from_tier(tier: ResearchQualityTier) -> Self {
        match tier {
            ResearchQualityTier::Low => Self {
                solver: SolverType::PBD,
                viscosity_solver: ViscositySolver::XSPH,
                quality_tier: tier,
                max_iterations: 4,
                enable_particle_shifting: false,
                enable_warm_start: false,
                enable_vorticity_confinement: false,
                enable_micropolar: false,
                ..Default::default()
            },
            ResearchQualityTier::Medium => Self {
                solver: SolverType::PCISPH,
                viscosity_solver: ViscositySolver::Morris,
                quality_tier: tier,
                max_iterations: 8,
                enable_particle_shifting: true,
                shifting_method: ShiftingMethod::StandardDelta,
                enable_warm_start: true,
                enable_vorticity_confinement: false,
                ..Default::default()
            },
            ResearchQualityTier::High => Self {
                // F.1: was SolverType::DFSPH (variant removed - never implemented).
                solver: SolverType::PCISPH,
                viscosity_solver: ViscositySolver::Morris,
                quality_tier: tier,
                max_iterations: 15,
                enable_particle_shifting: true,
                shifting_method: ShiftingMethod::StandardDelta,
                enable_warm_start: true,
                enable_vorticity_confinement: true,
                vorticity_epsilon: 0.05,
                ..Default::default()
            },
            ResearchQualityTier::Ultra => Self {
                // F.1: was SolverType::DFSPH (variant removed - never implemented).
                solver: SolverType::PCISPH,
                viscosity_solver: ViscositySolver::ImplicitJacobi,
                quality_tier: tier,
                max_iterations: 25,
                enable_particle_shifting: true,
                shifting_method: ShiftingMethod::InterfaceAware,
                enable_warm_start: true,
                enable_vorticity_confinement: true,
                vorticity_epsilon: 0.08,
                enable_micropolar: true,
                enable_implicit_air: true,
                ..Default::default()
            },
            ResearchQualityTier::Research => Self {
                // F.1: was SolverType::DFSPH (variant removed - never implemented).
                solver: SolverType::PCISPH,
                viscosity_solver: ViscositySolver::ImplicitJacobi,
                quality_tier: tier,
                kernel_type: KernelType::WendlandC4, // Higher fidelity for research
                max_iterations: 50,
                min_iterations: 5,
                density_error_threshold: 0.0001,
                enable_particle_shifting: true,
                shifting_method: ShiftingMethod::InterfaceAware,
                enable_warm_start: true,
                warm_start_factor: 0.9,
                enable_vorticity_confinement: true,
                vorticity_epsilon: 0.1,
                enable_micropolar: true,
                enable_implicit_air: true,
                enable_non_newtonian: true,
                export_metrics: true,
                metric_interval: 30,
                enable_vtk_export: true,
                ..Default::default()
            },
        }
    }

    /// Convert to GPU simulation parameters
    pub fn to_sim_params(&self, particle_count: u32) -> ResearchSimParams {
        ResearchSimParams {
            particle_count,
            max_iterations: self.max_iterations,
            min_iterations: self.min_iterations,
            density_error_threshold: self.density_error_threshold,
            warm_start_factor: self.warm_start_factor,
            vorticity_epsilon: self.vorticity_epsilon,
            solver_type: match self.solver {
                SolverType::PBD => 0,
                SolverType::PCISPH => 1,
            },
            kernel_type: match self.kernel_type {
                KernelType::CubicSpline => 0,
                KernelType::WendlandC2 => 1,
                KernelType::WendlandC4 => 2,
                KernelType::WendlandC6 => 3,
            },
            enable_shifting: u32::from(self.enable_particle_shifting),
            enable_vorticity: u32::from(self.enable_vorticity_confinement),
            enable_warm_start: u32::from(self.enable_warm_start),
            ..Default::default()
        }
    }
}

// ============================================================================
// VALIDATION METRICS
// ============================================================================

/// Validation metrics for research-grade simulation
#[derive(Clone, Debug, Default)]
pub struct ValidationMetrics {
    /// Maximum density error (|ρ - ρ₀| / ρ₀)
    pub density_error_max: f32,
    /// Average density error
    pub density_error_avg: f32,
    /// Maximum velocity divergence
    pub divergence_error_max: f32,
    /// Average velocity divergence
    pub divergence_error_avg: f32,
    /// Kinetic energy (0.5 * Σ m v²)
    pub kinetic_energy: f32,
    /// Potential energy (Σ m g h)
    pub potential_energy: f32,
    /// Total energy for conservation check
    pub total_energy: f32,
    /// Momentum conservation [x, y, z]
    pub momentum: [f32; 3],
    /// Mass conservation (should be constant)
    pub total_mass: f32,
    /// Number of solver iterations this frame
    pub solver_iterations: u32,
    /// Frame time in milliseconds
    pub frame_time_ms: f32,
}

// ============================================================================
// FLUID PHASE PRESETS
// ============================================================================

/// Fluid material preset with physical properties
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct FluidPhase {
    /// Phase identifier
    pub id: u32,
    /// Human-readable name
    pub name: String,
    /// Rest density (kg/m³)
    pub rest_density: f32,
    /// Dynamic viscosity (Pa·s)
    pub viscosity: f32,
    /// Surface tension coefficient (N/m)
    pub surface_tension: f32,
    /// Render color RGBA
    pub color: [f32; 4],
    /// Is this a gas phase?
    pub is_gas: bool,
}

impl FluidPhase {
    /// Water at 20°C
    pub fn water() -> Self {
        Self {
            id: 0,
            name: "Water".into(),
            rest_density: 1000.0,
            viscosity: 0.001,
            surface_tension: 0.0728,
            color: [0.2, 0.5, 0.9, 0.8],
            is_gas: false,
        }
    }

    /// Vegetable oil
    pub fn oil() -> Self {
        Self {
            id: 1,
            name: "Oil".into(),
            rest_density: 920.0,
            viscosity: 0.05,
            surface_tension: 0.032,
            color: [0.9, 0.8, 0.2, 0.9],
            is_gas: false,
        }
    }

    /// Honey
    pub fn honey() -> Self {
        Self {
            id: 2,
            name: "Honey".into(),
            rest_density: 1400.0,
            viscosity: 5.0,
            surface_tension: 0.05,
            color: [0.9, 0.6, 0.1, 1.0],
            is_gas: false,
        }
    }

    /// Air (for bubbles)
    pub fn air() -> Self {
        Self {
            id: 3,
            name: "Air".into(),
            rest_density: 1.2,
            viscosity: 0.000018,
            surface_tension: 0.0,
            color: [0.9, 0.9, 1.0, 0.3],
            is_gas: true,
        }
    }

    /// Glycerin
    pub fn glycerin() -> Self {
        Self {
            id: 4,
            name: "Glycerin".into(),
            rest_density: 1260.0,
            viscosity: 1.5,
            surface_tension: 0.063,
            color: [0.95, 0.95, 0.95, 0.95],
            is_gas: false,
        }
    }

    /// Lava (high temperature)
    pub fn lava() -> Self {
        Self {
            id: 5,
            name: "Lava".into(),
            rest_density: 2500.0,
            viscosity: 1000.0, // Highly temperature-dependent
            surface_tension: 0.4,
            color: [1.0, 0.3, 0.0, 1.0],
            is_gas: false,
        }
    }
}

// ============================================================================
// UNIT TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_research_particle_size() {
        // ResearchParticle size should be 16-byte aligned for GPU buffer compatibility
        let size = std::mem::size_of::<ResearchParticle>();
        // Print actual size for debugging
        println!("ResearchParticle size: {} bytes", size);

        // Must be at least 176 bytes for all required fields
        assert!(
            size >= 176,
            "ResearchParticle must be at least 176 bytes (got {} bytes)",
            size
        );
        // Must be 16-byte aligned
        assert_eq!(
            size % 16,
            0,
            "ResearchParticle size must be multiple of 16 for GPU buffer compatibility (got {} bytes)", size
        );
    }

    #[test]
    fn test_research_particle_alignment() {
        // Verify 16-byte alignment for GPU buffers
        assert_eq!(
            std::mem::align_of::<ResearchParticle>(),
            4, // f32 alignment
            "ResearchParticle alignment check"
        );
        // Size must be multiple of 16 for uniform buffer compatibility
        assert_eq!(
            std::mem::size_of::<ResearchParticle>() % 16,
            0,
            "ResearchParticle size must be multiple of 16"
        );
    }

    #[test]
    fn test_research_sim_params_size() {
        // Verify uniform buffer compatibility
        let size = std::mem::size_of::<ResearchSimParams>();
        assert_eq!(size % 16, 0, "ResearchSimParams must be 16-byte aligned");
    }

    #[test]
    fn test_default_config() {
        let config = ResearchFluidConfig::default();
        assert_eq!(config.solver, SolverType::PCISPH);
        assert!(config.enable_particle_shifting);
        assert!(config.enable_warm_start);
    }

    #[test]
    fn test_quality_tiers() {
        // Low tier should use PBD
        let low = ResearchFluidConfig::from_tier(ResearchQualityTier::Low);
        assert_eq!(low.solver, SolverType::PBD);
        assert!(!low.enable_particle_shifting);

        // High tier uses PCISPH with shifting (was DFSPH pre-F.1; that
        // variant was removed — no solver loop ever existed for it)
        let high = ResearchFluidConfig::from_tier(ResearchQualityTier::High);
        assert_eq!(high.solver, SolverType::PCISPH);
        assert!(high.enable_particle_shifting);
        assert!(high.enable_vorticity_confinement);

        // Research tier should have all features
        let research = ResearchFluidConfig::from_tier(ResearchQualityTier::Research);
        assert_eq!(research.solver, SolverType::PCISPH);
        assert!(research.enable_vtk_export);
        assert!(research.enable_micropolar);
    }

    #[test]
    fn test_solver_warm_start_support() {
        assert!(!SolverType::PBD.supports_warm_start());
        assert!(SolverType::PCISPH.supports_warm_start());
    }

    #[test]
    fn test_particle_creation() {
        let p = ResearchParticle::new(1.0, 2.0, 3.0);
        assert_eq!(p.position[0], 1.0);
        assert_eq!(p.position[1], 2.0);
        assert_eq!(p.position[2], 3.0);
        assert_eq!(p.density, 1000.0); // Default water density
    }

    #[test]
    fn test_fluid_phase_presets() {
        let water = FluidPhase::water();
        assert_eq!(water.rest_density, 1000.0);
        assert!(!water.is_gas);

        let air = FluidPhase::air();
        assert_eq!(air.rest_density, 1.2);
        assert!(air.is_gas);

        let honey = FluidPhase::honey();
        assert!(honey.viscosity > 1.0); // High viscosity
    }

    #[test]
    fn test_config_to_sim_params() {
        let config = ResearchFluidConfig::from_tier(ResearchQualityTier::High);
        let params = config.to_sim_params(100000);

        assert_eq!(params.particle_count, 100000);
        assert_eq!(params.solver_type, 1); // PCISPH (High tier post-F.1)
        assert_eq!(params.enable_shifting, 1);
        assert_eq!(params.enable_vorticity, 1);
        assert_eq!(params.enable_warm_start, 1);
    }

    // ==================== Additional Tests for Coverage ====================

    #[test]
    fn test_solver_type_typical_iterations() {
        assert_eq!(SolverType::PBD.typical_iterations(), 4);
        assert_eq!(SolverType::PCISPH.typical_iterations(), 5);
    }

    #[test]
    fn test_viscosity_solver_high_viscosity() {
        assert!(!ViscositySolver::XSPH.supports_high_viscosity());
        assert!(!ViscositySolver::Morris.supports_high_viscosity());
        assert!(ViscositySolver::ImplicitJacobi.supports_high_viscosity());
    }

    #[test]
    fn test_particle_with_phase() {
        let p = ResearchParticle::with_phase(1.0, 2.0, 3.0, 1, 920.0, 0.05);
        assert_eq!(p.position[0], 1.0);
        assert_eq!(p.phase, 1);
        assert_eq!(p.density, 920.0);
        assert!((p.viscosity_coefficient - 0.05).abs() < 1e-6);
    }

    #[test]
    fn test_particle_is_gas_phase() {
        let mut p = ResearchParticle::default();
        assert!(!p.is_gas_phase());

        p.is_gas = 1;
        assert!(p.is_gas_phase());
    }

    #[test]
    fn test_particle_is_at_surface() {
        let mut p = ResearchParticle::default();
        assert!(!p.is_at_surface());

        p.is_surface = 1;
        assert!(p.is_at_surface());
    }

    #[test]
    fn test_quality_tier_medium() {
        let medium = ResearchFluidConfig::from_tier(ResearchQualityTier::Medium);
        assert_eq!(medium.solver, SolverType::PCISPH);
        assert!(medium.enable_particle_shifting);
        assert!(medium.enable_warm_start);
        assert!(!medium.enable_vorticity_confinement);
    }

    #[test]
    fn test_quality_tier_ultra() {
        let ultra = ResearchFluidConfig::from_tier(ResearchQualityTier::Ultra);
        assert_eq!(ultra.solver, SolverType::PCISPH); // was DFSPH pre-F.1
        assert!(ultra.enable_micropolar);
        assert!(ultra.enable_implicit_air);
        assert_eq!(ultra.shifting_method, ShiftingMethod::InterfaceAware);
    }

    #[test]
    fn test_fluid_phase_oil() {
        let oil = FluidPhase::oil();
        assert_eq!(oil.id, 1);
        assert_eq!(oil.rest_density, 920.0);
        assert!(!oil.is_gas);
        assert!(oil.viscosity > 0.01);
    }

    #[test]
    fn test_fluid_phase_glycerin() {
        let glycerin = FluidPhase::glycerin();
        assert_eq!(glycerin.id, 4);
        assert!(glycerin.viscosity > 1.0);
    }

    #[test]
    fn test_fluid_phase_lava() {
        let lava = FluidPhase::lava();
        assert_eq!(lava.id, 5);
        assert!(lava.viscosity >= 1000.0);
        assert!(lava.rest_density > 2000.0);
    }

    #[test]
    fn test_research_sim_params_default() {
        let params = ResearchSimParams::default();
        assert_eq!(params.target_density, 1000.0);
        assert_eq!(params.gravity, -9.81);
        assert_eq!(params.solver_type, 1); // PCISPH
        assert_eq!(params.enable_shifting, 1);
    }

    #[test]
    fn test_validation_metrics_default() {
        let metrics = ValidationMetrics::default();
        assert_eq!(metrics.density_error_max, 0.0);
        assert_eq!(metrics.total_mass, 0.0);
        assert_eq!(metrics.solver_iterations, 0);
    }

    #[test]
    fn test_shifting_method_variants() {
        assert_eq!(ShiftingMethod::default(), ShiftingMethod::None);
        assert_ne!(
            ShiftingMethod::StandardDelta,
            ShiftingMethod::InterfaceAware
        );
    }

    #[test]
    fn test_shear_rate_method_variants() {
        assert_eq!(ShearRateMethod::default(), ShearRateMethod::VorticityBased);
        assert_ne!(ShearRateMethod::StrainTensor, ShearRateMethod::Blended);
    }

    #[test]
    fn test_boundary_method_variants() {
        assert_eq!(BoundaryMethod::default(), BoundaryMethod::Hybrid);
        assert_ne!(BoundaryMethod::AkinciOnly, BoundaryMethod::SDFOnly);
    }

    #[test]
    fn test_research_quality_tier_default() {
        assert_eq!(ResearchQualityTier::default(), ResearchQualityTier::Medium);
    }

    #[test]
    fn test_config_to_sim_params_pbd() {
        let config = ResearchFluidConfig::from_tier(ResearchQualityTier::Low);
        let params = config.to_sim_params(50000);

        assert_eq!(params.particle_count, 50000);
        assert_eq!(params.solver_type, 0); // PBD
        assert_eq!(params.enable_shifting, 0);
        assert_eq!(params.enable_vorticity, 0);
        assert_eq!(params.enable_warm_start, 0);
    }

    #[test]
    fn test_config_to_sim_params_pcisph() {
        // (Was test_config_to_sim_params_iisph; the IISPH variant was
        // removed in F.1 — no solver loop existed for it.)
        let mut config = ResearchFluidConfig::default();
        config.solver = SolverType::PCISPH;
        let params = config.to_sim_params(100);

        assert_eq!(params.solver_type, 1); // PCISPH
    }

    #[test]
    fn test_research_particle_default_values() {
        let p = ResearchParticle::default();
        assert_eq!(p.temperature, 293.0);
        assert_eq!(p.alpha, 0.0);
        assert_eq!(p.kappa, 0.0);
        assert_eq!(p.velocity_divergence, 0.0);
        assert_eq!(p.previous_pressure, 0.0);
        assert_eq!(p.shift_delta, [0.0; 3]);
        assert_eq!(p.vorticity, [0.0; 3]);
        assert_eq!(p.angular_velocity, [0.0; 3]);
        assert_eq!(p.phase_gradient, [0.0; 3]);
    }

    // ==================== Kernel Type Tests ====================

    #[test]
    fn test_kernel_type_default() {
        // Default should be WendlandC2 for stability
        assert_eq!(KernelType::default(), KernelType::WendlandC2);
    }

    #[test]
    fn test_kernel_type_continuity() {
        assert_eq!(KernelType::CubicSpline.continuity(), 2);
        assert_eq!(KernelType::WendlandC2.continuity(), 2);
        assert_eq!(KernelType::WendlandC4.continuity(), 4);
        assert_eq!(KernelType::WendlandC6.continuity(), 6);
    }

    #[test]
    fn test_kernel_type_stability() {
        // Cubic spline is NOT stable (pairing instability)
        assert!(!KernelType::CubicSpline.is_stable());

        // Wendland kernels ARE stable
        assert!(KernelType::WendlandC2.is_stable());
        assert!(KernelType::WendlandC4.is_stable());
        assert!(KernelType::WendlandC6.is_stable());
    }

    #[test]
    fn test_kernel_type_relative_cost() {
        // CubicSpline is baseline (1.0)
        assert!((KernelType::CubicSpline.relative_cost() - 1.0).abs() < 1e-6);

        // Higher smoothness = higher cost
        assert!(KernelType::WendlandC2.relative_cost() > KernelType::CubicSpline.relative_cost());
        assert!(KernelType::WendlandC4.relative_cost() > KernelType::WendlandC2.relative_cost());
        assert!(KernelType::WendlandC6.relative_cost() > KernelType::WendlandC4.relative_cost());
    }

    #[test]
    fn test_config_kernel_type_default() {
        let config = ResearchFluidConfig::default();
        assert_eq!(config.kernel_type, KernelType::WendlandC2);
    }

    #[test]
    fn test_config_to_sim_params_kernel_type() {
        // Default should use WendlandC2 (value 1)
        let config = ResearchFluidConfig::default();
        let params = config.to_sim_params(1000);
        assert_eq!(params.kernel_type, 1); // WendlandC2

        // Research tier should use WendlandC4 (value 2)
        let research = ResearchFluidConfig::from_tier(ResearchQualityTier::Research);
        let params = research.to_sim_params(1000);
        assert_eq!(params.kernel_type, 2); // WendlandC4
    }

    #[test]
    fn test_research_sim_params_kernel_type_default() {
        let params = ResearchSimParams::default();
        assert_eq!(params.kernel_type, 1); // WendlandC2
    }
}
