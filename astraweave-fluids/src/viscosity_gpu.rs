// =============================================================================
// GPU Viscosity Pipeline - Research-Grade Viscosity Solver
// =============================================================================
//
// Production-quality GPU viscosity pipeline using wgpu compute shaders.
// Supports Morris explicit, XSPH, and matrix-free implicit Jacobi methods.
//
// References:
// - Morris et al. 1997: "Modeling Low Reynolds Number Incompressible Flows"
// - Weiler et al. 2018: "Physically Consistent Implicit Viscosity Solver"
// - Peer et al. 2015: "Implicit SPH Formulation for Incompressible Solids"
//
// =============================================================================

use crate::research::ViscositySolver;

/// Compute workgroup size (must match shader)
pub const VISCOSITY_WORKGROUP_SIZE: u32 = 64;

/// Maximum implicit viscosity iterations
pub const MAX_IMPLICIT_ITERATIONS: u32 = 50;

/// Default implicit viscosity tolerance
pub const DEFAULT_IMPLICIT_TOLERANCE: f32 = 1e-4;

// =============================================================================
// GPU UNIFORM STRUCTURES
// =============================================================================

/// GPU-side viscosity parameters uniform buffer
///
/// Intended layout for a future viscosity shader. (The previously referenced
/// `viscosity_morris.wgsl` was an orphan with no consumer and was deleted in
/// F.1; this module remains CPU-side scaffolding behind `experimental`.)
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ViscosityParamsGpu {
    // Core simulation
    pub particle_count: u32,
    pub dt: f32,
    pub smoothing_radius: f32,
    pub particle_mass: f32,

    // Base viscosity
    pub base_viscosity: f32,
    pub cell_size: f32,
    pub grid_width: u32,
    pub grid_height: u32,

    // Grid and feature flags
    pub grid_depth: u32,
    pub enable_non_newtonian: u32,
    pub enable_temperature: u32,
    pub iteration: u32,

    // Non-Newtonian parameters (Carreau/Cross/Power-Law)
    pub non_newtonian_type: u32, // 0=Newtonian, 1=PowerLaw, 2=Carreau, 3=Cross, 4=Bingham
    pub viscosity_0: f32,        // Zero-shear viscosity
    pub viscosity_inf: f32,      // Infinite-shear viscosity
    pub power_index: f32,        // n

    pub lambda: f32,         // Relaxation time (Carreau)
    pub yield_stress: f32,   // τ_y (Bingham)
    pub cross_exponent: f32, // m (Cross model)
    _pad0: f32,

    // Temperature parameters
    pub temp_model_type: u32,   // 0=Constant, 1=Arrhenius, 2=VTF
    pub reference_temp: f32,    // T_ref (Kelvin)
    pub activation_energy: f32, // E_a (J/mol)
    _pad1: f32,

    pub temp_coefficient: f32, // B (VTF)
    pub vogel_temp: f32,       // T₀ (VTF)
    _pad2: f32,
    _pad3: f32,
}

impl Default for ViscosityParamsGpu {
    fn default() -> Self {
        Self {
            particle_count: 0,
            dt: 0.001,
            smoothing_radius: 1.2,
            particle_mass: 1.0,
            base_viscosity: 0.001,
            cell_size: 1.2,
            grid_width: 32,
            grid_height: 32,
            grid_depth: 32,
            enable_non_newtonian: 0,
            enable_temperature: 0,
            iteration: 0,
            non_newtonian_type: 0,
            viscosity_0: 0.001,
            viscosity_inf: 0.0001,
            power_index: 1.0,
            lambda: 1.0,
            yield_stress: 0.0,
            cross_exponent: 1.0,
            _pad0: 0.0,
            temp_model_type: 0,
            reference_temp: 293.0,
            activation_energy: 0.0,
            _pad1: 0.0,
            temp_coefficient: 0.0,
            vogel_temp: 0.0,
            _pad2: 0.0,
            _pad3: 0.0,
        }
    }
}

/// Configuration for the GPU viscosity system
#[derive(Clone, Debug)]
pub struct ViscosityGpuConfig {
    /// Which solver method to use
    pub solver: ViscositySolver,
    /// Base dynamic viscosity (Pa·s)
    pub base_viscosity: f32,
    /// Smoothing radius
    pub smoothing_radius: f32,
    /// Timestep
    pub dt: f32,
    /// Particle mass
    pub particle_mass: f32,
    /// Enable non-Newtonian behavior
    pub enable_non_newtonian: bool,
    /// Enable temperature-dependent viscosity
    pub enable_temperature: bool,
    /// Non-Newtonian type (0=Newtonian, 1=PowerLaw, 2=Carreau, 3=Cross, 4=Bingham)
    pub non_newtonian_type: u32,
    /// Zero-shear viscosity
    pub viscosity_0: f32,
    /// Infinite-shear viscosity
    pub viscosity_inf: f32,
    /// Power law index
    pub power_index: f32,
    /// Relaxation time (Carreau)
    pub lambda: f32,
    /// Yield stress (Bingham)
    pub yield_stress: f32,
    /// Cross model exponent
    pub cross_exponent: f32,
    /// Temperature model type (0=Constant, 1=Arrhenius, 2=VTF)
    pub temp_model_type: u32,
    /// Reference temperature (K)
    pub reference_temp: f32,
    /// Activation energy (J/mol) for Arrhenius
    pub activation_energy: f32,
    /// Temperature coefficient B for VTF
    pub temp_coefficient: f32,
    /// Vogel temperature T₀ for VTF
    pub vogel_temp: f32,
    /// Max iterations for implicit solver
    pub max_iterations: u32,
    /// Tolerance for implicit solver convergence
    pub tolerance: f32,
    /// SOR relaxation factor (0.5-1.0)
    pub omega: f32,
}

impl Default for ViscosityGpuConfig {
    fn default() -> Self {
        Self {
            solver: ViscositySolver::Morris,
            base_viscosity: 0.001,
            smoothing_radius: 1.2,
            dt: 0.001,
            particle_mass: 1.0,
            enable_non_newtonian: false,
            enable_temperature: false,
            non_newtonian_type: 0,
            viscosity_0: 0.001,
            viscosity_inf: 0.0001,
            power_index: 1.0,
            lambda: 1.0,
            yield_stress: 0.0,
            cross_exponent: 1.0,
            temp_model_type: 0,
            reference_temp: 293.0,
            activation_energy: 0.0,
            temp_coefficient: 0.0,
            vogel_temp: 0.0,
            max_iterations: 10,
            tolerance: 1e-4,
            omega: 0.8,
        }
    }
}

impl ViscosityGpuConfig {
    /// Create config for water at room temperature
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
            ..Default::default()
        }
    }

    /// Create config for honey
    pub fn honey() -> Self {
        Self {
            solver: ViscositySolver::ImplicitJacobi,
            base_viscosity: 5.0,
            max_iterations: 20,
            ..Default::default()
        }
    }

    /// Create config for shear-thinning fluid (ketchup, paint)
    pub fn shear_thinning() -> Self {
        Self {
            enable_non_newtonian: true,
            non_newtonian_type: 2, // Carreau
            viscosity_0: 1.0,
            viscosity_inf: 0.01,
            power_index: 0.3,
            lambda: 10.0,
            ..Default::default()
        }
    }

    /// Create config for shear-thickening fluid (cornstarch)
    pub fn shear_thickening() -> Self {
        Self {
            enable_non_newtonian: true,
            non_newtonian_type: 1, // Power law
            viscosity_0: 0.01,
            power_index: 1.5,
            ..Default::default()
        }
    }

    /// Convert to GPU uniform buffer
    pub fn to_gpu_params(
        &self,
        particle_count: u32,
        grid_dims: [u32; 3],
        cell_size: f32,
    ) -> ViscosityParamsGpu {
        ViscosityParamsGpu {
            particle_count,
            dt: self.dt,
            smoothing_radius: self.smoothing_radius,
            particle_mass: self.particle_mass,
            base_viscosity: self.base_viscosity,
            cell_size,
            grid_width: grid_dims[0],
            grid_height: grid_dims[1],
            grid_depth: grid_dims[2],
            enable_non_newtonian: self.enable_non_newtonian as u32,
            enable_temperature: self.enable_temperature as u32,
            iteration: 0,
            non_newtonian_type: self.non_newtonian_type,
            viscosity_0: self.viscosity_0,
            viscosity_inf: self.viscosity_inf,
            power_index: self.power_index,
            lambda: self.lambda,
            yield_stress: self.yield_stress,
            cross_exponent: self.cross_exponent,
            _pad0: 0.0,
            temp_model_type: self.temp_model_type,
            reference_temp: self.reference_temp,
            activation_energy: self.activation_energy,
            _pad1: 0.0,
            temp_coefficient: self.temp_coefficient,
            vogel_temp: self.vogel_temp,
            _pad2: 0.0,
            _pad3: 0.0,
        }
    }
}

// =============================================================================
// GPU VISCOSITY SYSTEM (Placeholder for wgpu integration)
// =============================================================================

/// Viscosity solve result statistics
#[derive(Clone, Debug, Default)]
pub struct ViscositySolveResult {
    /// Number of iterations (1 for explicit methods)
    pub iterations: u32,
    /// Final residual (for implicit solver)
    pub residual: f32,
    /// Time taken in milliseconds
    pub time_ms: f32,
    /// Maximum velocity change
    pub max_velocity_change: f32,
    /// Solver method used
    pub method: ViscositySolver,
}

/// Handle for the GPU viscosity pipeline
///
/// This struct manages all GPU resources for viscosity computation.
/// It supports three methods:
/// - XSPH: Simple velocity smoothing (for games)
/// - Morris: Explicit physically-accurate viscosity
/// - ImplicitJacobi: Matrix-free implicit solver for high viscosity
pub struct ViscosityGpuSystem {
    /// Configuration
    config: ViscosityGpuConfig,
    /// Current particle count
    particle_count: u32,
    /// Grid dimensions
    grid_dims: [u32; 3],
    /// Cell size
    cell_size: f32,
    /// Last solve result
    last_result: ViscositySolveResult,
    /// Whether system is initialized (used in GPU pipeline)
    #[allow(dead_code)]
    initialized: bool,
}

impl ViscosityGpuSystem {
    /// Create a new viscosity GPU system
    pub fn new(config: ViscosityGpuConfig) -> Self {
        Self {
            config,
            particle_count: 0,
            grid_dims: [32, 32, 32],
            cell_size: 1.2,
            last_result: ViscositySolveResult::default(),
            initialized: false,
        }
    }

    /// Get current configuration
    pub fn config(&self) -> &ViscosityGpuConfig {
        &self.config
    }

    /// Update configuration
    pub fn set_config(&mut self, config: ViscosityGpuConfig) {
        self.config = config;
    }

    /// Get last solve result
    pub fn last_result(&self) -> &ViscositySolveResult {
        &self.last_result
    }

    /// Set particle count for workgroup dispatch calculation
    pub fn set_particle_count(&mut self, count: u32) {
        self.particle_count = count;
    }

    /// Set grid dimensions
    pub fn set_grid_dims(&mut self, dims: [u32; 3], cell_size: f32) {
        self.grid_dims = dims;
        self.cell_size = cell_size;
    }

    /// Compute workgroup dispatch count
    pub fn workgroup_count(&self) -> u32 {
        self.particle_count.div_ceil(VISCOSITY_WORKGROUP_SIZE)
    }

    /// Generate GPU params for shader
    pub fn gpu_params(&self) -> ViscosityParamsGpu {
        self.config
            .to_gpu_params(self.particle_count, self.grid_dims, self.cell_size)
    }

    /// Get the shader entry point name for current solver
    pub fn shader_entry_point(&self) -> &'static str {
        match self.config.solver {
            ViscositySolver::XSPH => "compute_xsph_viscosity",
            ViscositySolver::Morris => "compute_morris_viscosity",
            ViscositySolver::ImplicitJacobi => "iterate_implicit_viscosity",
        }
    }

    /// Check if implicit solver is being used
    pub fn is_implicit(&self) -> bool {
        matches!(self.config.solver, ViscositySolver::ImplicitJacobi)
    }

    /// Get number of passes needed for implicit solver
    pub fn implicit_passes(&self) -> u32 {
        if self.is_implicit() {
            self.config.max_iterations
        } else {
            1
        }
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_viscosity_params_gpu_size() {
        // Ensure struct size matches shader expectations (must be 16-byte aligned)
        let size = std::mem::size_of::<ViscosityParamsGpu>();
        assert_eq!(size % 16, 0, "ViscosityParamsGpu must be 16-byte aligned");
    }

    #[test]
    fn test_viscosity_params_gpu_default() {
        let params = ViscosityParamsGpu::default();
        assert_eq!(params.particle_count, 0);
        assert!((params.dt - 0.001).abs() < 1e-6);
        assert!((params.base_viscosity - 0.001).abs() < 1e-6);
    }

    #[test]
    fn test_viscosity_gpu_config_water() {
        let config = ViscosityGpuConfig::water();
        assert!((config.base_viscosity - 0.001).abs() < 1e-6);
        assert!(!config.enable_non_newtonian);
    }

    #[test]
    fn test_viscosity_gpu_config_honey() {
        let config = ViscosityGpuConfig::honey();
        assert!(config.base_viscosity > 1.0);
        assert!(matches!(config.solver, ViscositySolver::ImplicitJacobi));
    }

    #[test]
    fn test_viscosity_gpu_config_shear_thinning() {
        let config = ViscosityGpuConfig::shear_thinning();
        assert!(config.enable_non_newtonian);
        assert_eq!(config.non_newtonian_type, 2); // Carreau
        assert!(config.power_index < 1.0);
    }

    #[test]
    fn test_viscosity_gpu_config_shear_thickening() {
        let config = ViscosityGpuConfig::shear_thickening();
        assert!(config.enable_non_newtonian);
        assert!(config.power_index > 1.0);
    }

    #[test]
    fn test_viscosity_gpu_config_to_params() {
        let config = ViscosityGpuConfig::water();
        let params = config.to_gpu_params(1000, [32, 32, 32], 1.2);
        assert_eq!(params.particle_count, 1000);
        assert_eq!(params.grid_width, 32);
        assert!((params.cell_size - 1.2).abs() < 1e-6);
    }

    #[test]
    fn test_viscosity_gpu_system_creation() {
        let config = ViscosityGpuConfig::default();
        let system = ViscosityGpuSystem::new(config);
        assert_eq!(system.particle_count, 0);
        assert!(!system.initialized);
    }

    #[test]
    fn test_viscosity_gpu_system_workgroup_count() {
        let config = ViscosityGpuConfig::default();
        let mut system = ViscosityGpuSystem::new(config);

        system.set_particle_count(1000);
        let count = system.workgroup_count();
        assert_eq!(count, (1000 + 63) / 64);

        system.set_particle_count(64);
        assert_eq!(system.workgroup_count(), 1);

        system.set_particle_count(65);
        assert_eq!(system.workgroup_count(), 2);
    }

    #[test]
    fn test_viscosity_gpu_system_shader_entry_points() {
        let mut config = ViscosityGpuConfig::default();

        config.solver = ViscositySolver::XSPH;
        let system = ViscosityGpuSystem::new(config.clone());
        assert_eq!(system.shader_entry_point(), "compute_xsph_viscosity");

        config.solver = ViscositySolver::Morris;
        let system = ViscosityGpuSystem::new(config.clone());
        assert_eq!(system.shader_entry_point(), "compute_morris_viscosity");

        config.solver = ViscositySolver::ImplicitJacobi;
        let system = ViscosityGpuSystem::new(config);
        assert_eq!(system.shader_entry_point(), "iterate_implicit_viscosity");
    }

    #[test]
    fn test_viscosity_gpu_system_implicit_passes() {
        let mut config = ViscosityGpuConfig::default();
        config.max_iterations = 15;

        config.solver = ViscositySolver::Morris;
        let system = ViscosityGpuSystem::new(config.clone());
        assert_eq!(system.implicit_passes(), 1);
        assert!(!system.is_implicit());

        config.solver = ViscositySolver::ImplicitJacobi;
        let system = ViscosityGpuSystem::new(config);
        assert_eq!(system.implicit_passes(), 15);
        assert!(system.is_implicit());
    }

    #[test]
    fn test_viscosity_solve_result_default() {
        let result = ViscositySolveResult::default();
        assert_eq!(result.iterations, 0);
        assert_eq!(result.residual, 0.0);
    }

    #[test]
    fn test_viscosity_gpu_system_config_access() {
        let config = ViscosityGpuConfig::oil();
        let mut system = ViscosityGpuSystem::new(config);

        assert!((system.config().base_viscosity - 0.05).abs() < 1e-6);

        system.set_config(ViscosityGpuConfig::honey());
        assert!(system.config().base_viscosity > 1.0);
    }

    #[test]
    fn test_viscosity_gpu_system_grid_dims() {
        let config = ViscosityGpuConfig::default();
        let mut system = ViscosityGpuSystem::new(config);

        system.set_grid_dims([64, 64, 64], 0.5);
        let params = system.gpu_params();
        assert_eq!(params.grid_width, 64);
        assert_eq!(params.grid_height, 64);
        assert_eq!(params.grid_depth, 64);
        assert!((params.cell_size - 0.5).abs() < 1e-6);
    }
}
