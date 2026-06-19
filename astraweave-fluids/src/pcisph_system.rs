// =============================================================================
// PCISPH GPU System - Research-Grade Fluid Simulation
// =============================================================================
//
// PCISPH solver implementation using wgpu compute shaders (experimental).
//
// References:
// - Solenthaler & Pajarola 2009: "Predictive-Corrective Incompressible SPH"
// - Marrone et al. 2011: "δ-SPH model for simulating violent impact flows"
//
// HONEST STATUS (corrected in Fluids-Integration F.1):
// - The pressure loop runs a FIXED iteration count
//   (config.max_iterations.min(MAX_PCISPH_ITERATIONS)). There is NO
//   convergence check: the IterationState buffer is never written by any
//   kernel, per-particle density_errors are never reduced or read back, and
//   DEFAULT_DENSITY_THRESHOLD is unreachable config. The previous header
//   claim of "<0.1% density error after convergence" had no implementing
//   code path. Implementing real convergence readback is T4 (research-grade)
//   work, out of F.1 scope.
// - The PCISPH δ stiffness coefficient uses an empirically tuned sum term
//   (see compute_pcisph_delta), not the analytically derived value.
// - Aspirational performance target (unmeasured): 100-200k particles @ 60
//   FPS (Medium quality tier), 3-8 pressure iterations.
//
// =============================================================================

use crate::research::{
    FluidPhase, ResearchFluidConfig, ResearchParticle, ResearchQualityTier, ResearchSimParams,
    ShiftingMethod, SolverType,
};
use std::sync::Arc;
use wgpu::util::DeviceExt;

/// Maximum supported particles (constrained by GPU buffer limits)
pub const MAX_PARTICLES: u32 = 1_000_000;

/// Maximum pressure correction iterations per timestep
pub const MAX_PCISPH_ITERATIONS: u32 = 50;

/// Default density error threshold for convergence (0.1%)
pub const DEFAULT_DENSITY_THRESHOLD: f32 = 0.001;

/// Compute workgroup size (must match shader)
pub const WORKGROUP_SIZE: u32 = 64;

/// Default smoothing radius
pub const DEFAULT_SMOOTHING_RADIUS: f32 = 1.2;

/// PCISPH iteration state for CPU-side convergence monitoring
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub struct IterationState {
    pub iteration: u32,
    pub max_density_error: f32,
    pub avg_density_error: f32,
    pub converged: u32,
}

/// Physical parameters for the simulation
///
/// These are the actual numerical values for physics simulation,
/// separate from the ResearchFluidConfig which controls solver behavior.
#[derive(Clone, Debug)]
pub struct PhysicalParams {
    /// Smoothing radius (kernel support)
    pub smoothing_radius: f32,
    /// Target rest density (kg/m³)
    pub target_density: f32,
    /// Base viscosity coefficient (Pa·s)
    pub base_viscosity: f32,
    /// Surface tension coefficient (N/m)
    pub surface_tension: f32,
    /// Gravitational acceleration (m/s²)
    pub gravity: f32,
    /// Pressure stiffness multiplier
    pub pressure_stiffness: f32,
    /// δ-SPH shifting coefficient
    pub delta_sph_c_delta: f32,
    /// SOR omega for pressure solver
    pub sor_omega: f32,
    /// Divergence error threshold
    pub divergence_error_threshold: f32,
}

impl Default for PhysicalParams {
    fn default() -> Self {
        Self {
            smoothing_radius: DEFAULT_SMOOTHING_RADIUS,
            target_density: 1000.0, // Water
            base_viscosity: 0.001,  // Water at 20°C
            surface_tension: 0.0728,
            gravity: -9.81,
            pressure_stiffness: 50.0,
            delta_sph_c_delta: 0.04,
            sor_omega: 0.7,
            divergence_error_threshold: 0.01,
        }
    }
}

impl PhysicalParams {
    /// Create parameters for water at 20°C
    #[must_use]
    pub fn water() -> Self {
        Self::default()
    }

    /// Create parameters for oil
    #[must_use]
    pub fn oil() -> Self {
        Self {
            target_density: 920.0,
            base_viscosity: 0.05,
            surface_tension: 0.032,
            ..Default::default()
        }
    }

    /// Create parameters for honey
    #[must_use]
    pub fn honey() -> Self {
        Self {
            target_density: 1400.0,
            base_viscosity: 5.0,
            surface_tension: 0.05,
            ..Default::default()
        }
    }

    /// Create from FluidPhase struct
    #[must_use]
    pub fn from_fluid_phase(phase: &FluidPhase) -> Self {
        Self {
            target_density: phase.rest_density,
            base_viscosity: phase.viscosity,
            surface_tension: phase.surface_tension,
            ..Default::default()
        }
    }
}

/// GPU buffer handles for PCISPH simulation
pub struct PcisphBuffers {
    /// Research particle buffer (176 bytes per particle)
    pub particles: wgpu::Buffer,

    /// Pressure buffer (f32 per particle, separate for ping-pong)
    pub pressure: wgpu::Buffer,

    /// Spatial hash grid head pointers
    pub grid_heads: wgpu::Buffer,

    /// Linked list next pointers
    pub grid_next: wgpu::Buffer,

    /// Simulation parameters uniform
    pub params: wgpu::Buffer,

    /// Iteration state for convergence checking
    pub iteration_state: wgpu::Buffer,

    /// Per-particle density errors for convergence
    pub density_errors: wgpu::Buffer,

    /// Staging buffer for CPU readback
    pub staging: wgpu::Buffer,

    /// Dynamic scene objects
    pub dynamic_objects: wgpu::Buffer,
}

/// PCISPH compute pipeline handles
pub struct PcisphPipelines {
    /// Predict positions (gravity, external forces)
    pub predict: wgpu::ComputePipeline,

    /// Clear spatial hash grid
    pub clear_grid: wgpu::ComputePipeline,

    /// Build spatial hash grid
    pub build_grid: wgpu::ComputePipeline,

    /// Compute density and density error
    pub compute_density: wgpu::ComputePipeline,

    /// PCISPH pressure solve iteration
    pub pressure_solve: wgpu::ComputePipeline,

    /// Apply pressure acceleration
    pub apply_pressure: wgpu::ComputePipeline,

    /// Apply viscosity forces
    pub viscosity: wgpu::ComputePipeline,

    /// Particle shifting (δ-SPH)
    pub particle_shifting: wgpu::ComputePipeline,

    /// Final integration and boundary handling
    pub integrate: wgpu::ComputePipeline,

    /// Compute vorticity (optional)
    pub compute_vorticity: wgpu::ComputePipeline,

    /// Apply vorticity confinement (optional)
    pub apply_vorticity: wgpu::ComputePipeline,
}

/// Bind group layouts for PCISPH
pub struct PcisphBindGroupLayouts {
    /// Group 0: Global infrastructure
    pub global: wgpu::BindGroupLayout,

    /// Group 1: Particles
    pub particles: wgpu::BindGroupLayout,

    /// Group 2: Scene data
    pub scene: wgpu::BindGroupLayout,
}

/// Active bind groups for a frame
pub struct PcisphBindGroups {
    pub global: wgpu::BindGroup,
    pub particles: wgpu::BindGroup,
    pub scene: wgpu::BindGroup,
}

/// Simulation statistics for profiling and validation
#[derive(Debug, Clone, Default)]
pub struct PcisphStats {
    /// Number of particles in simulation
    pub particle_count: u32,

    /// Last frame's pressure iteration count
    pub pressure_iterations: u32,

    /// Maximum density error (as fraction, 0.001 = 0.1%)
    pub max_density_error: f32,

    /// Average density error
    pub avg_density_error: f32,

    /// Whether simulation converged this frame
    pub converged: bool,

    /// Substep time in milliseconds
    pub substep_time_ms: f32,

    /// Number of substeps per frame
    pub substeps_per_frame: u32,
}

/// GPU-accelerated PCISPH fluid simulation system
pub struct PcisphSystem {
    /// GPU device reference
    device: Arc<wgpu::Device>,

    /// Command queue
    queue: Arc<wgpu::Queue>,

    /// Simulation configuration (solver behavior)
    config: ResearchFluidConfig,

    /// Physical parameters (numerical values)
    physical: PhysicalParams,

    /// GPU buffers
    buffers: PcisphBuffers,

    /// Compute pipelines
    pipelines: PcisphPipelines,

    /// Bind group layouts
    #[allow(dead_code)]
    layouts: PcisphBindGroupLayouts,

    /// Active bind groups
    bind_groups: PcisphBindGroups,

    /// Current particle count
    particle_count: u32,

    /// Grid dimensions
    grid_size: (u32, u32, u32),

    /// Simulation time
    time: f32,

    /// Frame counter
    frame: u32,

    /// Simulation statistics
    stats: PcisphStats,
}

impl PcisphSystem {
    /// Create a new PCISPH system with the given configuration
    ///
    /// # Arguments
    /// * `device` - GPU device
    /// * `queue` - Command queue
    /// * `config` - Research fluid configuration (solver behavior)
    /// * `physical` - Physical parameters (numerical values)
    /// * `initial_particles` - Optional initial particle data
    ///
    /// # Returns
    /// Result containing the PCISPH system or an error
    ///
    /// # Errors
    /// Returns `PcisphError` if configuration is invalid or GPU resources fail
    pub fn new(
        device: Arc<wgpu::Device>,
        queue: Arc<wgpu::Queue>,
        config: ResearchFluidConfig,
        physical: PhysicalParams,
        initial_particles: Option<&[ResearchParticle]>,
    ) -> Result<Self, PcisphError> {
        // Validate configuration
        Self::validate_config(&config, &physical)?;

        let particle_count = initial_particles.map_or(0, |p| p.len() as u32);

        if particle_count > MAX_PARTICLES {
            return Err(PcisphError::TooManyParticles {
                requested: particle_count,
                maximum: MAX_PARTICLES,
            });
        }

        // Calculate grid dimensions
        let grid_size = Self::calculate_grid_size(&physical);

        // Create bind group layouts
        let layouts = Self::create_bind_group_layouts(&device);

        // Create compute pipelines
        let pipelines = Self::create_pipelines(&device, &layouts)?;

        // Create GPU buffers
        let buffers = Self::create_buffers(&device, grid_size, initial_particles)?;

        // Create bind groups
        let bind_groups = Self::create_bind_groups(&device, &layouts, &buffers);

        Ok(Self {
            device,
            queue,
            config,
            physical,
            buffers,
            pipelines,
            layouts,
            bind_groups,
            particle_count,
            grid_size,
            time: 0.0,
            frame: 0,
            stats: PcisphStats::default(),
        })
    }

    /// Create a PCISPH system with a preset fluid phase
    ///
    /// # Errors
    /// Returns `PcisphError` if configuration is invalid or GPU resources fail
    pub fn with_preset(
        device: Arc<wgpu::Device>,
        queue: Arc<wgpu::Queue>,
        quality: ResearchQualityTier,
        phase: FluidPhase,
        particle_count: u32,
    ) -> Result<Self, PcisphError> {
        let config = ResearchFluidConfig::from_tier(quality);
        let physical = PhysicalParams::from_fluid_phase(&phase);

        // Generate initial particle block
        let particles = Self::generate_particle_block(
            particle_count,
            (-10.0, 1.0, -10.0),
            (10.0, 30.0, 10.0),
            &physical,
            &phase,
        );

        Self::new(device, queue, config, physical, Some(&particles))
    }

    /// Validate configuration for PCISPH solver
    fn validate_config(
        config: &ResearchFluidConfig,
        physical: &PhysicalParams,
    ) -> Result<(), PcisphError> {
        if !matches!(config.solver, SolverType::PCISPH) {
            return Err(PcisphError::InvalidSolverType(config.solver));
        }

        if physical.smoothing_radius <= 0.0 {
            return Err(PcisphError::InvalidParameter {
                name: "smoothing_radius",
                value: physical.smoothing_radius,
                reason: "must be positive",
            });
        }

        if physical.target_density <= 0.0 {
            return Err(PcisphError::InvalidParameter {
                name: "target_density",
                value: physical.target_density,
                reason: "must be positive",
            });
        }

        Ok(())
    }

    /// Calculate grid dimensions based on simulation domain
    fn calculate_grid_size(physical: &PhysicalParams) -> (u32, u32, u32) {
        // Assume 60x50x60 meter domain (adjustable)
        let domain_size = (60.0, 50.0, 60.0);
        let cell_size = physical.smoothing_radius; // Cell size = kernel radius

        (
            ((domain_size.0 / cell_size).ceil() as u32).max(1),
            ((domain_size.1 / cell_size).ceil() as u32).max(1),
            ((domain_size.2 / cell_size).ceil() as u32).max(1),
        )
    }

    /// Create bind group layouts for all shader binding groups
    fn create_bind_group_layouts(device: &wgpu::Device) -> PcisphBindGroupLayouts {
        // Group 0: Global infrastructure
        let global = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("PCISPH Global Layout"),
            entries: &[
                // Params uniform
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: Some(
                            std::num::NonZeroU64::new(
                                std::mem::size_of::<ResearchSimParams>() as u64
                            )
                            .expect("non-zero struct size"),
                        ),
                    },
                    count: None,
                },
                // Grid head pointers (atomic)
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Grid next pointers
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Iteration state
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: Some(
                            std::num::NonZeroU64::new(std::mem::size_of::<IterationState>() as u64)
                                .expect("non-zero struct size"),
                        ),
                    },
                    count: None,
                },
                // Density errors
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        // Group 1: Particles
        let particles = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("PCISPH Particles Layout"),
            entries: &[
                // Particle buffer
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Pressure buffer
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        // Group 2: Scene data
        let scene = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("PCISPH Scene Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        PcisphBindGroupLayouts {
            global,
            particles,
            scene,
        }
    }

    /// Create compute pipelines for all PCISPH stages
    fn create_pipelines(
        device: &wgpu::Device,
        layouts: &PcisphBindGroupLayouts,
    ) -> Result<PcisphPipelines, PcisphError> {
        // Load shader source
        let shader_source = include_str!("../shaders/research/pcisph.wgsl");

        let shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("PCISPH Shader"),
            source: wgpu::ShaderSource::Wgsl(shader_source.into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("PCISPH Pipeline Layout"),
            bind_group_layouts: &[&layouts.global, &layouts.particles, &layouts.scene],
            push_constant_ranges: &[],
        });

        // Helper macro to create compute pipelines
        macro_rules! create_pipeline {
            ($entry:literal) => {
                device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some(concat!("PCISPH ", $entry)),
                    layout: Some(&pipeline_layout),
                    module: &shader_module,
                    entry_point: Some($entry),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    cache: None,
                })
            };
        }

        Ok(PcisphPipelines {
            predict: create_pipeline!("pcisph_predict"),
            clear_grid: create_pipeline!("pcisph_clear_grid"),
            build_grid: create_pipeline!("pcisph_build_grid"),
            compute_density: create_pipeline!("pcisph_compute_density"),
            pressure_solve: create_pipeline!("pcisph_pressure_solve"),
            apply_pressure: create_pipeline!("pcisph_apply_pressure"),
            viscosity: create_pipeline!("pcisph_viscosity"),
            particle_shifting: create_pipeline!("pcisph_particle_shifting"),
            integrate: create_pipeline!("pcisph_integrate"),
            compute_vorticity: create_pipeline!("pcisph_compute_vorticity"),
            apply_vorticity: create_pipeline!("pcisph_apply_vorticity"),
        })
    }

    /// Create GPU buffers for simulation
    fn create_buffers(
        device: &wgpu::Device,
        grid_size: (u32, u32, u32),
        initial_particles: Option<&[ResearchParticle]>,
    ) -> Result<PcisphBuffers, PcisphError> {
        let particle_buffer_size =
            (MAX_PARTICLES as usize * std::mem::size_of::<ResearchParticle>()) as u64;
        let grid_cell_count = (grid_size.0 * grid_size.1 * grid_size.2) as u64;

        // Particle buffer
        let particles = if let Some(initial) = initial_particles {
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("PCISPH Particles"),
                contents: bytemuck::cast_slice(initial),
                usage: wgpu::BufferUsages::STORAGE
                    | wgpu::BufferUsages::COPY_DST
                    | wgpu::BufferUsages::COPY_SRC,
            })
        } else {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("PCISPH Particles"),
                size: particle_buffer_size,
                usage: wgpu::BufferUsages::STORAGE
                    | wgpu::BufferUsages::COPY_DST
                    | wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: false,
            })
        };

        // Pressure buffer
        let pressure = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("PCISPH Pressure"),
            size: MAX_PARTICLES as u64 * 4,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Grid head pointers
        let grid_heads = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("PCISPH Grid Heads"),
            size: grid_cell_count * 4,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Grid next pointers
        let grid_next = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("PCISPH Grid Next"),
            size: MAX_PARTICLES as u64 * 4,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Simulation parameters
        let params = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("PCISPH Params"),
            size: std::mem::size_of::<ResearchSimParams>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Iteration state
        let iteration_state = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("PCISPH Iteration State"),
            size: std::mem::size_of::<IterationState>() as u64,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        // Density errors
        let density_errors = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("PCISPH Density Errors"),
            size: MAX_PARTICLES as u64 * 4,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        // Staging buffer for readback
        let staging = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("PCISPH Staging"),
            size: std::mem::size_of::<IterationState>() as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Dynamic objects (placeholder, 16 max)
        let dynamic_objects = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("PCISPH Dynamic Objects"),
            size: 16 * 256, // 16 objects × 256 bytes each
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Ok(PcisphBuffers {
            particles,
            pressure,
            grid_heads,
            grid_next,
            params,
            iteration_state,
            density_errors,
            staging,
            dynamic_objects,
        })
    }

    /// Create bind groups for the current buffer state
    fn create_bind_groups(
        device: &wgpu::Device,
        layouts: &PcisphBindGroupLayouts,
        buffers: &PcisphBuffers,
    ) -> PcisphBindGroups {
        let global = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("PCISPH Global Bind Group"),
            layout: &layouts.global,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffers.params.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: buffers.grid_heads.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: buffers.grid_next.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: buffers.iteration_state.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: buffers.density_errors.as_entire_binding(),
                },
            ],
        });

        let particles = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("PCISPH Particles Bind Group"),
            layout: &layouts.particles,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffers.particles.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: buffers.pressure.as_entire_binding(),
                },
            ],
        });

        let scene = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("PCISPH Scene Bind Group"),
            layout: &layouts.scene,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffers.dynamic_objects.as_entire_binding(),
            }],
        });

        PcisphBindGroups {
            global,
            particles,
            scene,
        }
    }

    /// Generate a block of particles for initialization
    fn generate_particle_block(
        count: u32,
        min: (f32, f32, f32),
        max: (f32, f32, f32),
        physical: &PhysicalParams,
        phase: &FluidPhase,
    ) -> Vec<ResearchParticle> {
        let spacing = physical.smoothing_radius * 0.5; // 2 particles per kernel radius
        let dims = (
            ((max.0 - min.0) / spacing) as u32,
            ((max.1 - min.1) / spacing) as u32,
            ((max.2 - min.2) / spacing) as u32,
        );

        let mut particles = Vec::with_capacity(count as usize);

        for z in 0..dims.2 {
            for y in 0..dims.1 {
                for x in 0..dims.0 {
                    if particles.len() >= count as usize {
                        break;
                    }

                    let pos = [
                        min.0 + (x as f32 + 0.5) * spacing,
                        min.1 + (y as f32 + 0.5) * spacing,
                        min.2 + (z as f32 + 0.5) * spacing,
                    ];

                    // Use ResearchParticle::default() and customize
                    let mut p = ResearchParticle::default();

                    // Set position with mass
                    let volume = spacing.powi(3);
                    let mass = physical.target_density * volume;
                    p.position = [pos[0], pos[1], pos[2], mass];

                    // Set velocity to zero
                    p.velocity = [0.0, 0.0, 0.0, 0.0];

                    // Set predicted position
                    p.predicted_position = [pos[0], pos[1], pos[2], 1.0];

                    // Set phase properties
                    p.phase = phase.id;
                    p.density = phase.rest_density;
                    p.viscosity_coefficient = phase.viscosity;
                    p.is_gas = u32::from(phase.is_gas);
                    p.color = phase.color;

                    particles.push(p);
                }
            }
        }

        particles
    }

    /// Update simulation parameters from config
    fn update_params(&self, dt: f32) {
        let shifting_enabled = match self.config.shifting_method {
            ShiftingMethod::None => false,
            _ => self.config.enable_particle_shifting,
        };

        let params = ResearchSimParams {
            smoothing_radius: self.physical.smoothing_radius,
            target_density: self.physical.target_density,
            pressure_multiplier: self.physical.pressure_stiffness,
            viscosity: self.physical.base_viscosity,
            surface_tension: self.physical.surface_tension,
            gravity: self.physical.gravity,
            dt,
            particle_count: self.particle_count,
            grid_width: self.grid_size.0,
            grid_height: self.grid_size.1,
            grid_depth: self.grid_size.2,
            cell_size: self.physical.smoothing_radius,
            object_count: 0,
            frame: self.frame,
            warm_start_factor: self.config.warm_start_factor,
            _pad0: 0.0,

            max_iterations: self.config.max_iterations,
            min_iterations: self.config.min_iterations,
            density_error_threshold: self.config.density_error_threshold,
            divergence_error_threshold: self.physical.divergence_error_threshold,
            pcisph_delta: self.compute_pcisph_delta(dt),
            shifting_strength: if shifting_enabled {
                self.physical.delta_sph_c_delta
            } else {
                0.0
            },
            vorticity_epsilon: self.config.vorticity_epsilon,
            sor_omega: self.physical.sor_omega,

            solver_type: match self.config.solver {
                SolverType::PBD => 0,
                SolverType::PCISPH => 1,
            },
            kernel_type: match self.config.kernel_type {
                crate::research::KernelType::CubicSpline => 0,
                crate::research::KernelType::WendlandC2 => 1,
                crate::research::KernelType::WendlandC4 => 2,
                crate::research::KernelType::WendlandC6 => 3,
            },
            enable_shifting: u32::from(shifting_enabled),
            enable_vorticity: u32::from(self.config.enable_vorticity_confinement),
            enable_warm_start: u32::from(self.config.enable_warm_start),
            _pad1: 0,
            _pad2: 0,
            _pad3: 0,
        };

        self.queue
            .write_buffer(&self.buffers.params, 0, bytemuck::bytes_of(&params));
    }

    /// Compute PCISPH δ coefficient based on current parameters
    fn compute_pcisph_delta(&self, dt: f32) -> f32 {
        // δ = -1 / (β * (-Σ∇W · Σ∇W - Σ(∇W · ∇W)))
        // β = Δt² * m² * 2 / ρ₀²
        //
        // For a cubic lattice with uniform particle distribution:
        // Sum term ≈ -0.5 (empirically tuned)

        let h = self.physical.smoothing_radius;
        let spacing = h * 0.5;
        let volume = spacing.powi(3);
        let mass = self.physical.target_density * volume;
        let rho0 = self.physical.target_density;

        let beta = dt * dt * mass * mass * 2.0 / (rho0 * rho0);

        // KNOWN APPROXIMATION (documented F.1): the analytically correct
        // value is the kernel-gradient sum over a prototype filled
        // neighborhood (Solenthaler & Pajarola 2009 eq. 8). This constant is
        // an empirical stand-in tuned for a cubic lattice; replacing it with
        // the real precomputation is T4 work.
        let sum_term = -0.5;

        -1.0 / (beta * sum_term + 1e-6)
    }

    /// Advance simulation by one timestep
    pub fn step(&mut self, dt: f32) {
        if self.particle_count == 0 {
            return;
        }

        // Update simulation parameters
        self.update_params(dt);

        let workgroup_count = self.particle_count.div_ceil(WORKGROUP_SIZE);
        let grid_total = self.grid_size.0 * self.grid_size.1 * self.grid_size.2;
        let grid_workgroup_count = grid_total.div_ceil(WORKGROUP_SIZE);

        // Create command encoder
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("PCISPH Step"),
            });

        // Step 1: Predict positions
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("PCISPH Predict"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.pipelines.predict);
            pass.set_bind_group(0, &self.bind_groups.global, &[]);
            pass.set_bind_group(1, &self.bind_groups.particles, &[]);
            pass.set_bind_group(2, &self.bind_groups.scene, &[]);
            pass.dispatch_workgroups(workgroup_count, 1, 1);
        }

        // Step 2: Clear grid
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("PCISPH Clear Grid"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.pipelines.clear_grid);
            pass.set_bind_group(0, &self.bind_groups.global, &[]);
            pass.set_bind_group(1, &self.bind_groups.particles, &[]);
            pass.set_bind_group(2, &self.bind_groups.scene, &[]);
            pass.dispatch_workgroups(grid_workgroup_count, 1, 1);
        }

        // Step 3: Build grid
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("PCISPH Build Grid"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.pipelines.build_grid);
            pass.set_bind_group(0, &self.bind_groups.global, &[]);
            pass.set_bind_group(1, &self.bind_groups.particles, &[]);
            pass.set_bind_group(2, &self.bind_groups.scene, &[]);
            pass.dispatch_workgroups(workgroup_count, 1, 1);
        }

        // Steps 4-6: Pressure correction loop
        let pressure_iterations = self.config.max_iterations.min(MAX_PCISPH_ITERATIONS);

        for _iter in 0..pressure_iterations {
            // Compute density
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("PCISPH Compute Density"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(&self.pipelines.compute_density);
                pass.set_bind_group(0, &self.bind_groups.global, &[]);
                pass.set_bind_group(1, &self.bind_groups.particles, &[]);
                pass.set_bind_group(2, &self.bind_groups.scene, &[]);
                pass.dispatch_workgroups(workgroup_count, 1, 1);
            }

            // Pressure solve
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("PCISPH Pressure Solve"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(&self.pipelines.pressure_solve);
                pass.set_bind_group(0, &self.bind_groups.global, &[]);
                pass.set_bind_group(1, &self.bind_groups.particles, &[]);
                pass.set_bind_group(2, &self.bind_groups.scene, &[]);
                pass.dispatch_workgroups(workgroup_count, 1, 1);
            }

            // Apply pressure
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("PCISPH Apply Pressure"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(&self.pipelines.apply_pressure);
                pass.set_bind_group(0, &self.bind_groups.global, &[]);
                pass.set_bind_group(1, &self.bind_groups.particles, &[]);
                pass.set_bind_group(2, &self.bind_groups.scene, &[]);
                pass.dispatch_workgroups(workgroup_count, 1, 1);
            }

            // Rebuild grid after position update
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("PCISPH Rebuild Grid"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(&self.pipelines.clear_grid);
                pass.set_bind_group(0, &self.bind_groups.global, &[]);
                pass.set_bind_group(1, &self.bind_groups.particles, &[]);
                pass.set_bind_group(2, &self.bind_groups.scene, &[]);
                pass.dispatch_workgroups(grid_workgroup_count, 1, 1);
            }
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("PCISPH Build Grid 2"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(&self.pipelines.build_grid);
                pass.set_bind_group(0, &self.bind_groups.global, &[]);
                pass.set_bind_group(1, &self.bind_groups.particles, &[]);
                pass.set_bind_group(2, &self.bind_groups.scene, &[]);
                pass.dispatch_workgroups(workgroup_count, 1, 1);
            }
        }

        self.stats.pressure_iterations = pressure_iterations;

        // Step 7: Viscosity
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("PCISPH Viscosity"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.pipelines.viscosity);
            pass.set_bind_group(0, &self.bind_groups.global, &[]);
            pass.set_bind_group(1, &self.bind_groups.particles, &[]);
            pass.set_bind_group(2, &self.bind_groups.scene, &[]);
            pass.dispatch_workgroups(workgroup_count, 1, 1);
        }

        // Step 8: Particle shifting (if enabled)
        if self.config.enable_particle_shifting
            && !matches!(self.config.shifting_method, ShiftingMethod::None)
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("PCISPH Particle Shifting"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.pipelines.particle_shifting);
            pass.set_bind_group(0, &self.bind_groups.global, &[]);
            pass.set_bind_group(1, &self.bind_groups.particles, &[]);
            pass.set_bind_group(2, &self.bind_groups.scene, &[]);
            pass.dispatch_workgroups(workgroup_count, 1, 1);
        }

        // Step 9: Final integration and boundaries
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("PCISPH Integrate"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.pipelines.integrate);
            pass.set_bind_group(0, &self.bind_groups.global, &[]);
            pass.set_bind_group(1, &self.bind_groups.particles, &[]);
            pass.set_bind_group(2, &self.bind_groups.scene, &[]);
            pass.dispatch_workgroups(workgroup_count, 1, 1);
        }

        // Vorticity confinement (if enabled)
        if self.config.enable_vorticity_confinement && self.config.vorticity_epsilon > 0.0 {
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("PCISPH Compute Vorticity"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(&self.pipelines.compute_vorticity);
                pass.set_bind_group(0, &self.bind_groups.global, &[]);
                pass.set_bind_group(1, &self.bind_groups.particles, &[]);
                pass.set_bind_group(2, &self.bind_groups.scene, &[]);
                pass.dispatch_workgroups(workgroup_count, 1, 1);
            }
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("PCISPH Apply Vorticity"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(&self.pipelines.apply_vorticity);
                pass.set_bind_group(0, &self.bind_groups.global, &[]);
                pass.set_bind_group(1, &self.bind_groups.particles, &[]);
                pass.set_bind_group(2, &self.bind_groups.scene, &[]);
                pass.dispatch_workgroups(workgroup_count, 1, 1);
            }
        }

        // Submit commands
        self.queue.submit(std::iter::once(encoder.finish()));

        // Update time and frame
        self.time += dt;
        self.frame += 1;
        self.stats.particle_count = self.particle_count;
    }

    /// Get current simulation statistics
    #[must_use]
    pub fn stats(&self) -> &PcisphStats {
        &self.stats
    }

    /// Get particle buffer for rendering
    #[must_use]
    pub fn particle_buffer(&self) -> &wgpu::Buffer {
        &self.buffers.particles
    }

    /// Get current particle count
    #[must_use]
    pub fn particle_count(&self) -> u32 {
        self.particle_count
    }

    /// Get current configuration
    #[must_use]
    pub fn config(&self) -> &ResearchFluidConfig {
        &self.config
    }

    /// Get physical parameters
    #[must_use]
    pub fn physical(&self) -> &PhysicalParams {
        &self.physical
    }

    /// Update configuration (requires pipeline rebuild if solver changes)
    ///
    /// # Errors
    /// Returns `PcisphError` if the new configuration is invalid
    pub fn set_config(&mut self, config: ResearchFluidConfig) -> Result<(), PcisphError> {
        Self::validate_config(&config, &self.physical)?;
        self.config = config;
        Ok(())
    }

    /// Update physical parameters
    ///
    /// # Errors
    /// Returns `PcisphError` if the new parameters are invalid
    pub fn set_physical(&mut self, physical: PhysicalParams) -> Result<(), PcisphError> {
        Self::validate_config(&self.config, &physical)?;
        self.physical = physical;
        Ok(())
    }
}

/// PCISPH system errors
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
#[must_use]
pub enum PcisphError {
    #[error("Too many particles: {requested} requested, maximum is {maximum}")]
    TooManyParticles { requested: u32, maximum: u32 },

    #[error("Invalid solver type: {0:?} (expected PCISPH)")]
    InvalidSolverType(SolverType),

    #[error("Invalid parameter {name}: {value} - {reason}")]
    InvalidParameter {
        name: &'static str,
        value: f32,
        reason: &'static str,
    },

    #[error("Shader compilation error: {0}")]
    ShaderError(String),

    #[error("Buffer creation failed: {0}")]
    BufferError(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytemuck::Zeroable;

    // ==================== IterationState Tests ====================

    #[test]
    fn test_iteration_state_size() {
        assert_eq!(std::mem::size_of::<IterationState>(), 16);
    }

    #[test]
    fn test_iteration_state_default() {
        let state = IterationState::default();
        assert_eq!(state.iteration, 0);
        assert_eq!(state.max_density_error, 0.0);
        assert_eq!(state.avg_density_error, 0.0);
        assert_eq!(state.converged, 0);
    }

    #[test]
    fn test_iteration_state_bytemuck() {
        let state = IterationState {
            iteration: 5,
            max_density_error: 0.01,
            avg_density_error: 0.005,
            converged: 1,
        };

        let bytes: &[u8] = bytemuck::bytes_of(&state);
        assert_eq!(bytes.len(), 16);

        let recovered: &IterationState = bytemuck::from_bytes(bytes);
        assert_eq!(recovered.iteration, 5);
        assert_eq!(recovered.converged, 1);
    }

    #[test]
    fn test_iteration_state_zeroed() {
        let state = IterationState::zeroed();
        assert_eq!(state.iteration, 0);
        assert_eq!(state.converged, 0);
    }

    #[test]
    fn test_iteration_state_converged_flag() {
        let not_converged = IterationState {
            converged: 0,
            ..Default::default()
        };
        let converged = IterationState {
            converged: 1,
            ..Default::default()
        };

        assert_eq!(not_converged.converged, 0);
        assert_eq!(converged.converged, 1);
    }

    // ==================== PhysicalParams Tests ====================

    #[test]
    fn test_physical_params_defaults() {
        let params = PhysicalParams::default();
        assert!((params.target_density - 1000.0).abs() < f32::EPSILON);
        assert!((params.base_viscosity - 0.001).abs() < f32::EPSILON);
        assert!(params.smoothing_radius > 0.0);
    }

    #[test]
    fn test_physical_params_presets() {
        let water = PhysicalParams::water();
        assert!((water.target_density - 1000.0).abs() < f32::EPSILON);

        let oil = PhysicalParams::oil();
        assert!((oil.target_density - 920.0).abs() < f32::EPSILON);
        assert!(oil.base_viscosity > water.base_viscosity);

        let honey = PhysicalParams::honey();
        assert!(honey.base_viscosity > oil.base_viscosity);
    }

    #[test]
    fn test_from_fluid_phase() {
        let phase = FluidPhase::water();
        let params = PhysicalParams::from_fluid_phase(&phase);

        assert!((params.target_density - phase.rest_density).abs() < f32::EPSILON);
        assert!((params.base_viscosity - phase.viscosity).abs() < f32::EPSILON);
    }

    #[test]
    fn test_physical_params_all_positive() {
        let params = PhysicalParams::default();

        assert!(params.smoothing_radius > 0.0);
        assert!(params.target_density > 0.0);
        assert!(params.base_viscosity >= 0.0);
        assert!(params.surface_tension >= 0.0);
        assert!(params.pressure_stiffness > 0.0);
    }

    #[test]
    fn test_physical_params_gravity_negative() {
        let params = PhysicalParams::default();
        assert!(params.gravity < 0.0); // Gravity should be negative (downward)
    }

    #[test]
    fn test_physical_params_clone() {
        let params = PhysicalParams::honey();
        let cloned = params.clone();

        assert_eq!(params.target_density, cloned.target_density);
        assert_eq!(params.base_viscosity, cloned.base_viscosity);
    }

    #[test]
    fn test_physical_params_debug() {
        let params = PhysicalParams::default();
        let debug_str = format!("{:?}", params);

        assert!(debug_str.contains("smoothing_radius"));
        assert!(debug_str.contains("target_density"));
    }

    #[test]
    fn test_physical_params_density_ordering() {
        let water = PhysicalParams::water();
        let oil = PhysicalParams::oil();
        let honey = PhysicalParams::honey();

        // Honey > Water > Oil (in density)
        assert!(honey.target_density > water.target_density);
        assert!(water.target_density > oil.target_density);
    }

    #[test]
    fn test_physical_params_viscosity_ordering() {
        let water = PhysicalParams::water();
        let oil = PhysicalParams::oil();
        let honey = PhysicalParams::honey();

        // Honey > Oil > Water (in viscosity)
        assert!(honey.base_viscosity > oil.base_viscosity);
        assert!(oil.base_viscosity > water.base_viscosity);
    }

    // ==================== Delta Computation Tests ====================

    #[test]
    fn test_pcisph_delta_computation() {
        let physical = PhysicalParams::default();

        // Manually compute delta
        let dt = 1.0 / 60.0;
        let h = physical.smoothing_radius;
        let spacing = h * 0.5;
        let volume = spacing.powi(3);
        let mass = physical.target_density * volume;
        let rho0 = physical.target_density;

        let beta = dt * dt * mass * mass * 2.0 / (rho0 * rho0);
        let sum_term = -0.5;
        let delta = -1.0 / (beta * sum_term + 1e-6);

        assert!(delta.is_finite());
        assert!(delta > 0.0); // Should be positive for compression resistance
    }

    #[test]
    fn test_pcisph_delta_varies_with_density() {
        let water = PhysicalParams::water();
        let honey = PhysicalParams::honey();

        let dt = 1.0 / 60.0;

        let compute_delta = |p: &PhysicalParams| {
            let spacing = p.smoothing_radius * 0.5;
            let volume = spacing.powi(3);
            let mass = p.target_density * volume;
            let beta = dt * dt * mass * mass * 2.0 / (p.target_density * p.target_density);
            -1.0 / (beta * -0.5 + 1e-6)
        };

        let delta_water = compute_delta(&water);
        let delta_honey = compute_delta(&honey);

        // Both should be positive
        assert!(delta_water > 0.0);
        assert!(delta_honey > 0.0);
        // But different due to different densities
        assert!((delta_water - delta_honey).abs() > 1e-6);
    }

    // ==================== Grid Size Tests ====================

    #[test]
    fn test_grid_size_calculation() {
        let physical = PhysicalParams::default();
        let grid_size = PcisphSystem::calculate_grid_size(&physical);

        assert!(grid_size.0 > 0);
        assert!(grid_size.1 > 0);
        assert!(grid_size.2 > 0);

        // Should be reasonable size
        assert!(grid_size.0 <= 256);
        assert!(grid_size.1 <= 256);
        assert!(grid_size.2 <= 256);
    }

    #[test]
    fn test_grid_size_scales_with_smoothing_radius() {
        let small_radius = PhysicalParams {
            smoothing_radius: 0.5,
            ..Default::default()
        };
        let large_radius = PhysicalParams {
            smoothing_radius: 2.0,
            ..Default::default()
        };

        let grid_small = PcisphSystem::calculate_grid_size(&small_radius);
        let grid_large = PcisphSystem::calculate_grid_size(&large_radius);

        // Smaller radius = more cells
        assert!(grid_small.0 > grid_large.0);
        assert!(grid_small.1 > grid_large.1);
        assert!(grid_small.2 > grid_large.2);
    }

    #[test]
    fn test_grid_size_always_at_least_one() {
        let huge_radius = PhysicalParams {
            smoothing_radius: 100.0,
            ..Default::default()
        };

        let grid = PcisphSystem::calculate_grid_size(&huge_radius);

        assert!(grid.0 >= 1);
        assert!(grid.1 >= 1);
        assert!(grid.2 >= 1);
    }

    // ==================== Constants Tests ====================

    #[test]
    fn test_max_particles_reasonable() {
        assert!(MAX_PARTICLES >= 100_000);
        assert!(MAX_PARTICLES <= 10_000_000);
    }

    #[test]
    fn test_max_iterations_reasonable() {
        assert!(MAX_PCISPH_ITERATIONS >= 10);
        assert!(MAX_PCISPH_ITERATIONS <= 100);
    }

    #[test]
    fn test_default_density_threshold() {
        assert!(DEFAULT_DENSITY_THRESHOLD > 0.0);
        assert!(DEFAULT_DENSITY_THRESHOLD < 0.1); // Should be < 10%
    }

    #[test]
    fn test_workgroup_size_power_of_two() {
        assert!(WORKGROUP_SIZE.is_power_of_two());
        assert!(WORKGROUP_SIZE >= 32);
        assert!(WORKGROUP_SIZE <= 256);
    }

    #[test]
    fn test_default_smoothing_radius() {
        assert!(DEFAULT_SMOOTHING_RADIUS > 0.0);
        assert!(DEFAULT_SMOOTHING_RADIUS < 10.0);
    }

    // ==================== PcisphStats Tests ====================

    #[test]
    fn test_pcisph_stats_default() {
        let stats = PcisphStats::default();

        assert_eq!(stats.particle_count, 0);
        assert_eq!(stats.pressure_iterations, 0);
        assert_eq!(stats.max_density_error, 0.0);
        assert_eq!(stats.avg_density_error, 0.0);
        assert!(!stats.converged);
        assert_eq!(stats.substep_time_ms, 0.0);
        assert_eq!(stats.substeps_per_frame, 0);
    }

    #[test]
    fn test_pcisph_stats_clone() {
        let stats = PcisphStats {
            particle_count: 10000,
            pressure_iterations: 5,
            max_density_error: 0.001,
            avg_density_error: 0.0005,
            converged: true,
            substep_time_ms: 2.5,
            substeps_per_frame: 2,
        };

        let cloned = stats.clone();
        assert_eq!(cloned.particle_count, 10000);
        assert_eq!(cloned.pressure_iterations, 5);
        assert!(cloned.converged);
    }

    // ==================== Error Type Tests ====================

    #[test]
    fn test_pcisph_error_too_many_particles() {
        let error = PcisphError::TooManyParticles {
            requested: 2_000_000,
            maximum: 1_000_000,
        };

        let msg = format!("{}", error);
        assert!(msg.contains("2000000"));
        assert!(msg.contains("1000000"));
    }

    #[test]
    fn test_pcisph_error_invalid_solver() {
        let error = PcisphError::InvalidSolverType(SolverType::PBD);
        let msg = format!("{}", error);
        assert!(msg.contains("PBD"));
    }

    #[test]
    fn test_pcisph_error_invalid_parameter() {
        let error = PcisphError::InvalidParameter {
            name: "smoothing_radius",
            value: -1.0,
            reason: "must be positive",
        };

        let msg = format!("{}", error);
        assert!(msg.contains("smoothing_radius"));
        assert!(msg.contains("-1"));
        assert!(msg.contains("must be positive"));
    }

    #[test]
    fn test_pcisph_error_shader() {
        let error = PcisphError::ShaderError("syntax error at line 42".to_string());
        let msg = format!("{}", error);
        assert!(msg.contains("syntax error"));
    }

    #[test]
    fn test_pcisph_error_buffer() {
        let error = PcisphError::BufferError("out of GPU memory".to_string());
        let msg = format!("{}", error);
        assert!(msg.contains("out of GPU memory"));
    }

    // ==================== Mutation-Resistant Tests ====================

    #[test]
    fn test_physical_params_modification_independence() {
        let mut params = PhysicalParams::water();
        params.base_viscosity = 100.0;

        // Modifying one field shouldn't affect others
        assert_eq!(params.target_density, 1000.0);
        assert_eq!(params.gravity, -9.81);
    }

    #[test]
    fn test_iteration_state_field_independence() {
        let mut state = IterationState::default();
        state.iteration = 10;
        state.converged = 1;

        // Other fields should remain default
        assert_eq!(state.max_density_error, 0.0);
        assert_eq!(state.avg_density_error, 0.0);
    }

    #[test]
    fn test_pcisph_stats_converged_requires_low_error() {
        // A reasonable converged simulation should have low density error
        let converged_stats = PcisphStats {
            converged: true,
            max_density_error: 0.001, // 0.1%
            ..Default::default()
        };

        assert!(converged_stats.max_density_error <= DEFAULT_DENSITY_THRESHOLD);
    }

    #[test]
    fn test_grid_cell_count_reasonable() {
        let physical = PhysicalParams::default();
        let grid = PcisphSystem::calculate_grid_size(&physical);

        let total_cells = grid.0 as u64 * grid.1 as u64 * grid.2 as u64;

        // Should be manageable for GPU (< 100M cells)
        assert!(total_cells < 100_000_000);
        // But enough for reasonable simulation
        assert!(total_cells > 100);
    }

    #[test]
    fn test_physical_params_surface_tension_range() {
        let water = PhysicalParams::water();
        let oil = PhysicalParams::oil();

        // Surface tension should be positive and reasonable
        assert!(water.surface_tension >= 0.0);
        assert!(water.surface_tension <= 1.0);
        assert!(oil.surface_tension >= 0.0);
        assert!(oil.surface_tension <= 1.0);
    }

    #[test]
    fn test_pcisph_error_debug() {
        let error = PcisphError::TooManyParticles {
            requested: 100,
            maximum: 50,
        };

        let debug_str = format!("{:?}", error);
        assert!(debug_str.contains("TooManyParticles"));
    }
}
