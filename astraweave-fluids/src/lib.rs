//! # AstraWeave Fluids
//!
//! Production-quality fluid simulation system for games, featuring:
//!
//! ## Core Systems
//! - **Position-Based Dynamics (PBD)** - GPU-accelerated particle simulation
//!
//! ## Visual Effects
//! - **Caustics** - Underwater light refraction patterns
//! - **God Rays** - Volumetric light shafts through water
//! - **Reflections** - Screen-space and planar water reflections
//! - **Foam** - Dynamic whitecaps, wakes, and shore foam
//! - **Particles** - Waterfalls, bubbles, debris, and spray
//!
//! ## Production Features
//! - Quality presets (Low/Medium/High/Ultra)
//! - LOD system with distance-based culling
//! - Profiling and stats collection
//! - Serialization for save/load
//! - Editor integration
//!
//! ## Quick Start
//! ```ignore
//! use astraweave_fluids::{WaterEffectsManager, WaterQualityPreset};
//!
//! let manager = WaterEffectsManager::from_preset(WaterQualityPreset::High)?;
//! manager.update(delta_time, camera_pos, water_height);
//! ```

// Suppress test-specific clippy lints from mutation-resistant test patterns
#![cfg_attr(
    test,
    allow(
        clippy::field_reassign_with_default,
        clippy::assertions_on_constants,
        clippy::manual_range_contains,
        clippy::useless_vec,
        clippy::clone_on_copy,
        clippy::needless_range_loop,
        clippy::manual_div_ceil,
        clippy::manual_abs_diff,
        clippy::nonminimal_bool,
        clippy::len_zero,
        clippy::const_is_empty,
        clippy::inefficient_to_string,
        non_snake_case,
        unused_variables,
    )
)]

pub mod anisotropic;
pub mod caustics;
pub mod debug_viz;
pub mod editor;
pub mod emitter;
pub mod foam;
pub mod god_rays;
pub mod lod;
pub mod optimization;
pub mod profiling;
pub mod renderer;
pub mod sdf;
pub mod serialization;
pub mod underwater;
pub mod underwater_particles;
pub mod water_effects;
pub mod water_reflections;
pub mod waterfall;

pub use caustics::{
    CausticSample, CausticsConfig, CausticsProjector, CausticsSystem, CausticsUniforms,
    CAUSTICS_WGSL,
};
pub use debug_viz::{
    DebugDrawList, DebugLine, DebugPoint, DebugVertex, ParticleDebugType, StatsFormatter,
    WaterDebugConfig,
};
pub use editor::{
    editor_shortcuts,
    AccessibilitySettings,
    // Batch operations for multi-select
    BatchOperation,
    CausticsEditorConfig,
    // Accessibility
    ColorblindPalette,
    // Clipboard support
    ConfigClipboard,
    // History & undo/redo support
    ConfigHistory,
    ConfigTransition,
    ConfigValidator,
    // Debug visualization
    DebugVisualization,
    DrainEditorConfig,
    // Animation easing
    EasingFunction,
    EditorMetadata,
    EmitterEditorConfig,
    EmitterShapeType,
    // Preset export/import
    ExportedPreset,
    FieldMetadata,
    FlowConfig,
    // Scene integration
    FluidAABB,
    FluidEditorConfig,
    // Performance profiling
    FluidPerformanceMetrics,
    FluidScenePlacement,
    FoamEditorConfig,
    GodRaysEditorConfig,
    // Keyboard shortcuts
    KeyboardShortcut,
    LodConfig as EditorLodConfig,
    PhysicsConfig,
    // Real-time preview hints
    PreviewHint,
    QualityPreset as EditorQualityPreset,
    ReflectionEditorConfig,
    RenderingConfig,
    ThermalConfig,
    UnderwaterEditorConfig,
    ValidationIssue,
    // Validation system
    ValidationSeverity,
    WaterBodyPreset,
    WaterfallEditorConfig,
    WaveConfig,
    // Widget metadata for UI generation
    WidgetType,
};
pub use emitter::{EmitterShape, FluidDrain, FluidEmitter};
pub use foam::{FoamConfig, FoamParticle, FoamSource, FoamSystem, FoamTrail, GpuFoamParticle};
pub use god_rays::{GodRaysConfig, GodRaysSystem, GodRaysUniforms, LightShaft, GOD_RAYS_WGSL};
pub use lod::{
    FluidLodConfig, FluidLodManager, LodLevel, LodUpdateResult, OptimizedLodConfig,
    OptimizedLodManager, ParticleStreamingManager, StreamingOp,
};
pub use optimization::{
    analyze_metrics, AdaptiveIterations, BatchSpawner, GpuShaderConfig, GpuVendor, MortonCode,
    OptimizationMetrics, OptimizationPreset, OptimizationProfiler, OptimizationRecommendation,
    OptimizedSimParams, ParticleStateGpu, QualityTier, SimulationBudget, TemporalCoherence,
    WorkgroupConfig,
};
pub use profiling::{FluidProfiler, FluidTimingStats};
pub use renderer::FluidRenderer;
pub use serialization::{FluidSnapshot, SnapshotParams};
pub use underwater::{DepthZoneManager, UnderwaterConfig, UnderwaterState, UnderwaterUniforms};
pub use underwater_particles::{
    BubbleStream, GpuUnderwaterParticle, UnderwaterParticle, UnderwaterParticleConfig,
    UnderwaterParticleSystem, UnderwaterParticleType,
};
pub use water_effects::{
    WaterEffectsConfig, WaterEffectsError, WaterEffectsManager, WaterEffectsResult,
    WaterEffectsStats, WaterQualityPreset,
};
pub use water_reflections::{
    PlanarReflection, ReflectionUniforms, WaterReflectionConfig, WaterReflectionSystem, SSR_WGSL,
};
pub use waterfall::{
    GpuWaterParticle, RapidsSystem, WaterParticle, WaterParticleType, WaterfallConfig,
    WaterfallSource, WaterfallSystem,
};

use std::borrow::Cow;
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Particle {
    pub position: [f32; 4],
    pub velocity: [f32; 4],
    pub predicted_position: [f32; 4],
    pub lambda: f32,
    pub density: f32,
    pub phase: u32,       // 0=water, 1=oil, 2=custom phase
    pub temperature: f32, // Kelvin (ambient ~293K)
    pub color: [f32; 4],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct DynamicObject {
    pub transform: [[f32; 4]; 4],
    pub inv_transform: [[f32; 4]; 4],
    pub half_extents: [f32; 4], // w = type (0=box, 1=sphere)
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SimParams {
    pub smoothing_radius: f32,
    pub target_density: f32,
    /// Scales the vorticity-confinement gain in `integrate` — NOT the XSPH
    /// viscosity blend, which is hardcoded at 0.01 in the shader. (A former
    /// `pressure_multiplier` field was removed in F.1: no kernel ever read
    /// it; PBF incompressibility comes from the lambda constraint, not a
    /// pressure stiffness.)
    pub viscosity: f32,
    pub surface_tension: f32,
    pub gravity: f32,
    pub dt: f32,
    pub particle_count: u32,
    pub grid_width: u32,
    pub grid_height: u32,
    pub grid_depth: u32,
    pub cell_size: f32,
    pub object_count: u32,
    pub _pad0: f32,
    pub _pad1: f32,
    pub _pad2: f32,
    pub _pad3: f32,
}

pub struct FluidSystem {
    /// The single particle storage buffer. All compute kernels mutate it in
    /// place; dispatch boundaries act as barriers between pipeline stages.
    /// (A vestigial second "ping-pong" buffer was removed in F.1: the shader
    /// never wrote it, so alternating bind groups simulated two divergent
    /// particle states on alternating frames.)
    particle_buffer: wgpu::Buffer,

    // Optimized Bind Groups
    global_bind_group: wgpu::BindGroup,
    particles_bind_group: wgpu::BindGroup,
    secondary_bind_group: wgpu::BindGroup,
    // Group 3 (Scene) is handled per-frame since SDF view can change

    // Optimized Bind Group Layouts
    #[allow(dead_code)]
    global_layout: wgpu::BindGroupLayout,
    #[allow(dead_code)]
    particles_layout: wgpu::BindGroupLayout,
    #[allow(dead_code)]
    secondary_layout: wgpu::BindGroupLayout,
    #[allow(dead_code)]
    scene_layout: wgpu::BindGroupLayout,

    #[allow(dead_code)]
    head_pointers: wgpu::Buffer,
    #[allow(dead_code)]
    next_pointers: wgpu::Buffer,

    clear_grid_pipeline: wgpu::ComputePipeline,
    build_grid_pipeline: wgpu::ComputePipeline,
    predict_pipeline: wgpu::ComputePipeline,
    lambda_pipeline: wgpu::ComputePipeline,
    delta_pos_pipeline: wgpu::ComputePipeline,
    integrate_pipeline: wgpu::ComputePipeline,
    mix_dye_pipeline: wgpu::ComputePipeline,

    params_buffer: wgpu::Buffer,
    pub particle_count: u32,
    pub frame_index: usize,

    // Sim constants
    pub smoothing_radius: f32,
    pub target_density: f32,
    pub viscosity: f32,
    pub surface_tension: f32,
    pub gravity: f32,
    pub iterations: u32,

    // Grid params
    pub cell_size: f32,
    pub grid_width: u32,
    pub grid_height: u32,
    pub grid_depth: u32,

    pub sdf_system: crate::sdf::SdfSystem,
    pub objects_buffer: wgpu::Buffer,
    /// Number of valid entries in `objects_buffer` (set by `update_objects`).
    object_count: u32,
    pub default_sampler: wgpu::Sampler,
    secondary_particle_buffer: wgpu::Buffer,
    secondary_counter: wgpu::Buffer,
    density_error_buffer: wgpu::Buffer,
    density_error_staging_buffers: [wgpu::Buffer; 2],
    staging_state: [StagingState; 2],
    staging_map_result: [std::sync::Arc<std::sync::atomic::AtomicU8>; 2],

    /// Optional GPU pass-timing instrumentation (see `enable_gpu_timing`).
    gpu_timing: Option<GpuStepTiming>,

    // Dynamic Particle Management
    particle_flags: wgpu::Buffer, // 0=inactive, 1=active for each particle
    pub active_count: u32,        // Currently active particles
    pub max_particles: u32,       // Buffer capacity
    free_list: Vec<u32>,          // CPU-side free list for respawning
    pending_despawn_regions: Vec<([f32; 3], [f32; 3])>, // (min, max) AABB regions to despawn
    /// CPU-side cache of particle positions for despawn region checks.
    /// Updated during spawn/reset. Note: becomes stale during GPU simulation,
    /// but provides approximate positions for culling. For exact GPU positions,
    /// a full GPU compute shader approach would be needed.
    particle_positions: Vec<[f32; 3]>,
    /// Flags indicating which particles are active (CPU-side mirror of particle_flags)
    particle_active: Vec<bool>,

    // ==================== OPTIMIZATION COMPONENTS ====================
    /// Workgroup configuration for GPU dispatch (vendor-aware)
    pub workgroup_config: WorkgroupConfig,
    /// Adaptive iteration controller based on density error
    pub adaptive_iterations: AdaptiveIterations,
    /// Frame time budget controller
    pub simulation_budget: SimulationBudget,
    /// Temporal coherence for skipping resting particles
    pub temporal_coherence: TemporalCoherence,
    /// Batch spawner for efficient particle creation
    pub batch_spawner: BatchSpawner,
    /// Optimization statistics
    pub optimization_stats: OptimizationStats,
}

/// Lifecycle of a density-error staging buffer (see `FluidSystem::step`).
///
/// The map request is only ever issued for a buffer whose copy has already
/// been *submitted* (i.e. at the start of a later `step()` call), so the
/// mapping can never observe unsubmitted work.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum StagingState {
    /// No copy in flight; safe to record a new copy into.
    Idle,
    /// A copy into this buffer was recorded; its encoder may not be
    /// submitted yet. Do not map.
    CopyRecorded,
    /// `map_async` has been issued (strictly after the copy's encoder was
    /// submitted, per the `step` contract). Harvest when the callback fires.
    MapRequested,
}

/// `map_async` callback outcome for a staging buffer.
const MAP_PENDING: u8 = 0;
const MAP_OK: u8 = 1;
const MAP_ERR: u8 = 2;

/// Y coordinate that despawned particles are parked at. Inactive particles
/// are skipped by every compute kernel (never simulated, never inserted into
/// the neighbor grid); parking them far below the world keeps them out of
/// any naive instanced render of the full buffer.
const DESPAWN_PARK_Y: f32 = -10_000.0;

/// Named GPU timing spans recorded by `FluidSystem::step` when
/// `enable_gpu_timing` has been called (WI-6 instrumentation).
///
/// `pbd_iterations` covers the whole lambda/delta-pos loop (all iterations);
/// `sdf` covers the SDF init→JFA→finalize chain (excluding the rare B→A
/// blit). Two query slots (begin/end) per span.
pub const GPU_TIMING_SPANS: [&str; 7] = [
    "sdf",
    "predict",
    "clear_grid",
    "build_grid",
    "pbd_iterations",
    "integrate",
    "mix_dye",
];

/// GPU pass-timing state (timestamp query set + readback buffers).
struct GpuStepTiming {
    query_set: wgpu::QuerySet,
    resolve_buffer: wgpu::Buffer,
    staging_buffer: wgpu::Buffer,
    /// Nanoseconds per timestamp tick (`Queue::get_timestamp_period`).
    period_ns: f32,
}

/// Statistics from optimization systems
#[derive(Clone, Debug, Default)]
pub struct OptimizationStats {
    /// Current quality level (0.0 - 1.0)
    pub quality_level: f32,
    /// Current iteration count
    pub iterations: u32,
    /// Number of resting particles skipped
    pub resting_particles: u32,
    /// Recommended iterations from budget controller
    pub recommended_iterations: u32,
    /// Whether simulation is under budget
    pub under_budget: bool,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SecondaryParticle {
    pub position: [f32; 4],
    pub velocity: [f32; 4],
    pub info: [f32; 4], // x: lifetime, y: type, z: alpha, w: scale
}

impl FluidSystem {
    /// Create a fluid system with `particle_count` particles arranged in a
    /// cubic grid.
    ///
    /// # Device requirements
    /// The internal SDF pipeline binds ReadOnly/WriteOnly storage textures on
    /// `Rgba32Float`, so `device` must be created with
    /// `wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES`.
    pub fn new(device: &wgpu::Device, particle_count: u32) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Fluid Shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("../shaders/fluid.wgsl"))),
        });

        // Create Buffers
        let mut initial_particles = Vec::with_capacity(particle_count as usize);
        let mut initial_positions = Vec::with_capacity(particle_count as usize);
        let spacing = 0.5;
        let size = (particle_count as f32).powf(1.0 / 3.0).ceil() as usize;

        for i in 0..particle_count as usize {
            let x = (i % size) as f32 * spacing - 5.0;
            let y = ((i / size) % size) as f32 * spacing + 2.0;
            let z = (i / (size * size)) as f32 * spacing - 5.0;

            initial_positions.push([x, y, z]);
            initial_particles.push(Particle {
                position: [x, y, z, 1.0],
                velocity: [0.0; 4],
                predicted_position: [x, y, z, 1.0],
                lambda: 0.0,
                density: 0.0,
                phase: 0,
                temperature: 293.0,
                color: [0.2, 0.5, 0.8, 1.0],
            });
        }

        let particle_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Particle Buffer"),
            contents: bytemuck::cast_slice(&initial_particles),
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::VERTEX
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
        });

        // Grid parameters
        let grid_width = 128u32;
        let grid_height = 128u32;
        let grid_depth = 128u32;
        let cell_size = 1.2; // Slightly larger than smoothing_radius
        let _grid_size = (grid_width * grid_height * grid_depth) as usize;

        let params = SimParams {
            dt: 0.016,
            smoothing_radius: 1.0,
            target_density: 12.0,
            viscosity: 10.0,
            surface_tension: 0.02,
            particle_count,
            gravity: -9.8,
            cell_size,
            grid_width,
            grid_height,
            grid_depth,
            object_count: 0,
            _pad0: 0.0,
            _pad1: 0.0,
            _pad2: 0.0,
            _pad3: 0.0,
        };

        let params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Sim Params Buffer"),
            contents: bytemuck::bytes_of(&params),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // --- Bind Group Layouts ---

        // Group 0: Global Infrastructure (Params, Counters, Pointers)
        let global_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Fluid Global Layout"),
            entries: &[
                // 0: SimParams
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // 1: Head Pointers
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
                // 2: Next Pointers
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
                // 3: Secondary Counter
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // 4: Density Error
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

        // Group 1: Particles (0: particle storage, 1: active flags)
        let particles_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Fluid Particles Layout"),
            entries: &[
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
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        // Group 2: Secondary Particles
        let secondary_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Fluid Secondary Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        // Group 3: Scene Data (Objects, SDF)
        let scene_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Fluid Scene Layout"),
            entries: &[
                // 0: Objects
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // 1: SDF Texture
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D3,
                        multisampled: false,
                    },
                    count: None,
                },
                // 2: Sampler
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
                    count: None,
                },
            ],
        });

        let default_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Fluid Default Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        // --- Infrastructure Buffers ---
        let head_pointers = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Head Pointers"),
            size: (grid_width * grid_height * grid_depth * 4) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let next_pointers = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Next Pointers"),
            size: (particle_count * 4) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let secondary_particle_count = 65536;
        let secondary_particle_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Secondary Particle Buffer"),
            size: (secondary_particle_count * std::mem::size_of::<SecondaryParticle>()) as u64,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::VERTEX,
            mapped_at_creation: false,
        });

        let secondary_counter = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Secondary Counter"),
            size: 4,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let density_error_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Density Error Buffer"),
            size: 4,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let density_error_staging_buffers = [
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Density Error Staging Buffer 0"),
                size: 4,
                usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Density Error Staging Buffer 1"),
                size: 4,
                usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
        ];

        // Dynamic Particle Management: flags buffer (1=active, 0=inactive)
        let initial_flags: Vec<u32> = vec![1u32; particle_count as usize];
        let particle_flags = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Particle Flags Buffer"),
            contents: bytemuck::cast_slice(&initial_flags),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        let objects_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Dynamic Objects Buffer"),
            size: (128 * std::mem::size_of::<DynamicObject>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // --- Pre-allocate Bind Groups ---
        let global_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Fluid Global BG"),
            layout: &global_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: params_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: head_pointers.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: next_pointers.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: secondary_counter.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: density_error_buffer.as_entire_binding(),
                },
            ],
        });

        let particles_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Particles BG"),
            layout: &particles_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: particle_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: particle_flags.as_entire_binding(),
                },
            ],
        });

        let secondary_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Secondary BG"),
            layout: &secondary_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: secondary_particle_buffer.as_entire_binding(),
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Fluid Pipeline Layout"),
            bind_group_layouts: &[
                &global_layout,
                &particles_layout,
                &secondary_layout,
                &scene_layout,
            ],
            push_constant_ranges: &[],
        });

        let (
            clear_grid_pipeline,
            build_grid_pipeline,
            predict_pipeline,
            lambda_pipeline,
            delta_pos_pipeline,
            integrate_pipeline,
            mix_dye_pipeline,
        ) = {
            let create_p = |label, entry_point| {
                device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some(label),
                    layout: Some(&pipeline_layout),
                    module: &shader,
                    entry_point: Some(entry_point),
                    compilation_options: Default::default(),
                    cache: None,
                })
            };
            (
                create_p("Clear Grid", "clear_grid"),
                create_p("Build Grid", "build_grid"),
                create_p("Predict", "predict"),
                create_p("Lambda", "compute_lambda"),
                create_p("Delta Pos", "compute_delta_pos"),
                create_p("Integrate", "integrate"),
                create_p("Mix Dye", "mix_dye"),
            )
        };

        let sdf_system = crate::sdf::SdfSystem::new(device, &objects_buffer, 64, 60.0);

        Self {
            particle_buffer,
            global_bind_group,
            particles_bind_group,
            secondary_bind_group,
            global_layout,
            particles_layout,
            secondary_layout,
            scene_layout,
            head_pointers,
            next_pointers,
            clear_grid_pipeline,
            build_grid_pipeline,
            predict_pipeline,
            lambda_pipeline,
            delta_pos_pipeline,
            integrate_pipeline,
            params_buffer,
            particle_count,
            frame_index: 0,
            smoothing_radius: 1.0,
            target_density: 12.0,
            viscosity: 10.0,
            surface_tension: 0.02,
            gravity: -9.8,
            iterations: 4,
            cell_size,
            grid_width,
            grid_height,
            grid_depth,
            sdf_system,
            objects_buffer,
            object_count: 0,
            default_sampler,
            secondary_particle_buffer,
            secondary_counter,
            density_error_buffer,
            density_error_staging_buffers,
            staging_state: [StagingState::Idle; 2],
            staging_map_result: [
                std::sync::Arc::new(std::sync::atomic::AtomicU8::new(MAP_PENDING)),
                std::sync::Arc::new(std::sync::atomic::AtomicU8::new(MAP_PENDING)),
            ],
            gpu_timing: None,
            mix_dye_pipeline,
            particle_flags,
            active_count: particle_count,
            max_particles: particle_count,
            free_list: Vec::new(),
            pending_despawn_regions: Vec::new(),
            particle_positions: initial_positions,
            particle_active: vec![true; particle_count as usize],

            // Initialize optimization components with sensible defaults
            workgroup_config: WorkgroupConfig::universal(),
            adaptive_iterations: AdaptiveIterations::new(2, 8),
            simulation_budget: SimulationBudget::new(8.0), // 8ms budget (half of 60fps frame)
            temporal_coherence: TemporalCoherence::new(0.01, 5),
            batch_spawner: BatchSpawner::new(1024),
            optimization_stats: OptimizationStats::default(),
        }
    }

    /// Enable per-pass GPU timestamp instrumentation (WI-6).
    ///
    /// Returns `false` (and stays disabled) if the device was not created
    /// with `wgpu::Features::TIMESTAMP_QUERY`. Once enabled, every `step()`
    /// records begin/end timestamps for the spans in [`GPU_TIMING_SPANS`]
    /// and resolves them into a staging buffer; read them with
    /// [`Self::read_gpu_timings`].
    pub fn enable_gpu_timing(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) -> bool {
        if !device.features().contains(wgpu::Features::TIMESTAMP_QUERY) {
            return false;
        }
        let slot_count = (GPU_TIMING_SPANS.len() * 2) as u32;
        let query_set = device.create_query_set(&wgpu::QuerySetDescriptor {
            label: Some("Fluid Step Timing"),
            ty: wgpu::QueryType::Timestamp,
            count: slot_count,
        });
        let size = (slot_count as u64) * 8;
        let resolve_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Fluid Timing Resolve"),
            size,
            usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Fluid Timing Staging"),
            size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.gpu_timing = Some(GpuStepTiming {
            query_set,
            resolve_buffer,
            staging_buffer,
            period_ns: queue.get_timestamp_period(),
        });
        true
    }

    /// Read back the most recent step's per-pass GPU timings in milliseconds.
    ///
    /// **Blocking** (`device.poll(Wait)`): intended for benches, tests, and
    /// diagnostics overlays — not the per-frame hot path. Call after the
    /// `step()` encoder has been submitted. Returns `None` if timing was
    /// never enabled or the mapping fails.
    pub fn read_gpu_timings(&self, device: &wgpu::Device) -> Option<Vec<(&'static str, f32)>> {
        let timing = self.gpu_timing.as_ref()?;
        let slice = timing.staging_buffer.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |res| {
            let _ = tx.send(res);
        });
        let _ = device.poll(wgpu::MaintainBase::Wait);
        rx.recv().ok()?.ok()?;
        let timestamps: Vec<u64> = {
            let data = slice.get_mapped_range();
            bytemuck::cast_slice(&data).to_vec()
        };
        timing.staging_buffer.unmap();

        let mut out = Vec::with_capacity(GPU_TIMING_SPANS.len());
        for (i, name) in GPU_TIMING_SPANS.iter().enumerate() {
            let begin = timestamps[i * 2];
            let end = timestamps[i * 2 + 1];
            let ms = if end > begin {
                (end - begin) as f32 * timing.period_ns / 1.0e6
            } else {
                0.0
            };
            out.push((*name, ms));
        }
        Some(out)
    }

    /// Build `timestamp_writes` for span `i` of [`GPU_TIMING_SPANS`].
    /// `begin`/`end` select which edge(s) this pass records (a multi-pass
    /// span puts `begin` on its first pass and `end` on its last).
    fn span_writes(
        &self,
        i: usize,
        begin: bool,
        end: bool,
    ) -> Option<wgpu::ComputePassTimestampWrites<'_>> {
        // wgpu rejects timestamp_writes with neither edge set (the interior
        // passes of a multi-pass span request no writes at all).
        if !begin && !end {
            return None;
        }
        self.gpu_timing
            .as_ref()
            .map(|t| wgpu::ComputePassTimestampWrites {
                query_set: &t.query_set,
                beginning_of_pass_write_index: begin.then_some((i * 2) as u32),
                end_of_pass_write_index: end.then_some((i * 2 + 1) as u32),
            })
    }

    /// Upload dynamic collider objects. The objects buffer holds at most 128
    /// entries; the count is propagated to both the SDF seed pass and the
    /// per-frame `SimParams` so the shaders only read the valid prefix.
    pub fn update_objects(&mut self, queue: &wgpu::Queue, objects: &[DynamicObject]) {
        let count = objects.len().min(128);
        if count > 0 {
            queue.write_buffer(
                &self.objects_buffer,
                0,
                bytemuck::cast_slice(&objects[..count]),
            );
        }
        self.object_count = count as u32;
        self.sdf_system.set_object_count(queue, self.object_count);
    }

    pub fn reset_particles(&mut self, queue: &wgpu::Queue, particles: &[Particle]) {
        assert_eq!(particles.len() as u32, self.particle_count);
        queue.write_buffer(&self.particle_buffer, 0, bytemuck::cast_slice(particles));
        // Reset all flags to active
        let flags: Vec<u32> = vec![1u32; particles.len()];
        queue.write_buffer(&self.particle_flags, 0, bytemuck::cast_slice(&flags));
        self.active_count = particles.len() as u32;
        self.free_list.clear();

        // Update CPU-side position cache
        self.particle_positions.clear();
        self.particle_positions.extend(
            particles
                .iter()
                .map(|p| [p.position[0], p.position[1], p.position[2]]),
        );
        self.particle_active = vec![true; particles.len()];
        self.pending_despawn_regions.clear();
    }

    /// Spawn new particles at runtime. Returns the number of particles actually spawned.
    /// Particles are spawned from the free list if available, or fails if at capacity.
    pub fn spawn_particles(
        &mut self,
        queue: &wgpu::Queue,
        positions: &[[f32; 3]],
        velocities: &[[f32; 3]],
        colors: Option<&[[f32; 4]]>,
    ) -> usize {
        let count = positions.len().min(velocities.len());
        let spawned = count.min(self.free_list.len());

        for i in 0..spawned {
            let idx = self.free_list.pop().expect("free_list must not be empty") as usize;
            let pos = positions[i];
            let vel = velocities[i];
            let color = colors.map(|c| c[i]).unwrap_or([0.2, 0.5, 0.8, 1.0]);

            let particle = Particle {
                position: [pos[0], pos[1], pos[2], 1.0],
                velocity: [vel[0], vel[1], vel[2], 0.0],
                predicted_position: [pos[0], pos[1], pos[2], 1.0],
                lambda: 0.0,
                density: 0.0,
                phase: 0,
                temperature: 293.0,
                color,
            };

            let offset = (idx * std::mem::size_of::<Particle>()) as u64;
            queue.write_buffer(&self.particle_buffer, offset, bytemuck::bytes_of(&particle));

            // Set flag to active
            let flag_offset = (idx * 4) as u64;
            queue.write_buffer(&self.particle_flags, flag_offset, bytemuck::bytes_of(&1u32));

            // Update CPU-side cache
            self.particle_positions[idx] = pos;
            self.particle_active[idx] = true;
        }

        self.active_count += spawned as u32;
        spawned
    }

    /// Despawn all particles within the given axis-aligned bounding box.
    ///
    /// This queues the region for processing in the next `step()` call.
    /// The actual despawn count is tracked internally and affects `active_count`.
    ///
    /// # Implementation Notes
    /// **Region membership is tested against the CPU-side position cache, which
    /// is exact only at spawn/reset time and goes stale as GPU simulation moves
    /// particles.** A particle that has drifted out of (or into) the AABB since
    /// it was spawned may be missed (or wrongly culled). For culling volumes
    /// much larger than the expected drift this is acceptable; for precise
    /// despawn an exact GPU-side region test would be required (future work).
    ///
    /// Once despawned, the effect IS exact: the particle's GPU flag is cleared,
    /// every compute kernel skips it, `build_grid` never inserts it into the
    /// neighbor grid, and it is parked far below the world.
    ///
    /// # Arguments
    /// * `_queue` - GPU queue (reserved for future GPU-accelerated implementation)
    /// * `min` - Minimum corner of the AABB `[x, y, z]`
    /// * `max` - Maximum corner of the AABB `[x, y, z]`
    ///
    /// # Returns
    /// The number of regions queued (always 1 for a single call).
    /// Use `active_count()` after `step()` to observe the effect.
    pub fn despawn_region(&mut self, _queue: &wgpu::Queue, min: [f32; 3], max: [f32; 3]) -> usize {
        // Validate AABB (ensure min <= max for each axis)
        let valid_min = [min[0].min(max[0]), min[1].min(max[1]), min[2].min(max[2])];
        let valid_max = [min[0].max(max[0]), min[1].max(max[1]), min[2].max(max[2])];

        self.pending_despawn_regions.push((valid_min, valid_max));
        1 // One region queued
    }

    /// Returns the number of pending despawn regions queued for processing.
    pub fn pending_despawn_count(&self) -> usize {
        self.pending_despawn_regions.len()
    }

    /// Clears all pending despawn regions without processing them.
    pub fn clear_pending_despawns(&mut self) {
        self.pending_despawn_regions.clear();
    }

    /// Process all pending despawn regions and mark particles within them as inactive.
    /// Returns the number of particles despawned.
    fn process_pending_despawns(&mut self, queue: &wgpu::Queue) -> usize {
        if self.pending_despawn_regions.is_empty() {
            return 0;
        }

        let mut despawned = 0;

        // Check each particle against each pending region
        for idx in 0..self.particle_positions.len() {
            // Skip already inactive particles
            if !self.particle_active[idx] {
                continue;
            }

            let pos = self.particle_positions[idx];

            // Check if position is inside any pending despawn region
            let should_despawn = self.pending_despawn_regions.iter().any(|(min, max)| {
                pos[0] >= min[0]
                    && pos[0] <= max[0]
                    && pos[1] >= min[1]
                    && pos[1] <= max[1]
                    && pos[2] >= min[2]
                    && pos[2] <= max[2]
            });

            if should_despawn {
                // Mark as inactive on GPU. Every compute kernel early-outs on
                // flag==0 and build_grid never inserts inactive particles, so
                // despawned particles stop participating in the simulation.
                let flag_offset = (idx * 4) as u64;
                queue.write_buffer(&self.particle_flags, flag_offset, bytemuck::bytes_of(&0u32));

                // Park the particle far below the world so naive instanced
                // renders of the full buffer don't draw a frozen ghost.
                let parked = Particle {
                    position: [0.0, DESPAWN_PARK_Y, 0.0, 0.0],
                    velocity: [0.0; 4],
                    predicted_position: [0.0, DESPAWN_PARK_Y, 0.0, 0.0],
                    lambda: 0.0,
                    density: 0.0,
                    phase: 0,
                    temperature: 293.0,
                    color: [0.0; 4],
                };
                let offset = (idx * std::mem::size_of::<Particle>()) as u64;
                queue.write_buffer(&self.particle_buffer, offset, bytemuck::bytes_of(&parked));

                // Update CPU-side state
                self.particle_active[idx] = false;
                self.free_list.push(idx as u32);
                despawned += 1;
            }
        }

        // Update active count
        if despawned > 0 {
            self.active_count = self.active_count.saturating_sub(despawned as u32);
        }

        // Clear processed regions
        self.pending_despawn_regions.clear();

        despawned
    }

    /// Record one simulation step into `encoder`.
    ///
    /// # Contract
    /// The encoder passed to each `step()` call **must be submitted before the
    /// next `step()` call** on the same `FluidSystem`. The density-error
    /// readback relies on this: a buffer whose copy was recorded in step N has
    /// its `map_async` issued at the start of step N+1 (i.e. strictly after
    /// submission), and is harvested when the mapping completes — typically at
    /// the start of step N+2.
    ///
    /// # Defined adaptive-iteration semantics
    /// `self.iterations` for frame N is derived from the smoothed average
    /// density error of the most recently *harvested* frame (normally N-2).
    /// This two-frame lag is the defined behavior; it makes the feedback loop
    /// race-free (no mapping of unsubmitted work). Note that the *particle
    /// state itself* is non-deterministic per the engine's fluids determinism
    /// carve-out (GPU atomic neighbor-list ordering), so the error value — and
    /// therefore the iteration count — may differ between identical runs.
    pub fn step(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        queue: &wgpu::Queue,
        dt: f32,
    ) {
        // Advance the density-error readback state machine: request mappings
        // for buffers whose copies are now submitted, and harvest any buffer
        // whose mapping has completed (updates `self.iterations`).
        self.pump_density_error_readback(device);

        // Process any pending despawn regions first
        let _despawned = self.process_pending_despawns(queue);

        // Update Uniforms
        let params = SimParams {
            smoothing_radius: self.smoothing_radius,
            target_density: self.target_density,
            viscosity: self.viscosity,
            surface_tension: self.surface_tension,
            gravity: self.gravity,
            dt: dt.min(0.016),
            particle_count: self.particle_count,
            grid_width: self.grid_width,
            grid_height: self.grid_height,
            grid_depth: self.grid_depth,
            cell_size: self.cell_size,
            object_count: self.object_count,
            _pad0: 0.0,
            _pad1: 0.0,
            _pad2: 0.0,
            _pad3: 0.0,
        };
        queue.write_buffer(&self.params_buffer, 0, bytemuck::bytes_of(&params));

        // 1. Generate SDF (timing span 0 brackets init -> finalize)
        self.sdf_system.generate_timed(
            encoder,
            queue,
            self.gpu_timing.as_ref().map(|t| (&t.query_set, 0u32, 1u32)),
        );

        let particle_workgroups = self.particle_count.div_ceil(64);

        // --- Setup Bind Groups ---
        let global_bg = &self.global_bind_group;
        let particles_bg = &self.particles_bind_group;
        let secondary_bg = &self.secondary_bind_group;

        // Group 3: Scene Data (Objects + SDF)
        // We create this per frame because the SDF texture view might change
        let sdf_view = self.sdf_system.texture_a.create_view(&Default::default());
        let scene_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Fluid Scene BG"),
            layout: &self.scene_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.objects_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&sdf_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&self.default_sampler),
                },
            ],
        });

        // 0. Reset density error and counters
        encoder.clear_buffer(&self.density_error_buffer, 0, None);
        encoder.clear_buffer(&self.secondary_counter, 0, None);

        // --- Execute Compute Pipeline ---

        // Common Bindings: 0:Global, 1:Particles, 2:Secondary, 3:Scene

        // 1. Predict and Clear Grid
        {
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Fluid::Predict"),
                timestamp_writes: self.span_writes(1, true, true),
            });
            cpass.set_pipeline(&self.predict_pipeline);
            cpass.set_bind_group(0, global_bg, &[]);
            cpass.set_bind_group(1, particles_bg, &[]);
            cpass.set_bind_group(2, secondary_bg, &[]);
            cpass.set_bind_group(3, &scene_bg, &[]);
            cpass.dispatch_workgroups(particle_workgroups, 1, 1);
        }

        {
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Fluid::ClearGrid"),
                timestamp_writes: self.span_writes(2, true, true),
            });
            cpass.set_pipeline(&self.clear_grid_pipeline);
            cpass.set_bind_group(0, global_bg, &[]);
            cpass.set_bind_group(1, particles_bg, &[]);
            cpass.set_bind_group(2, secondary_bg, &[]);
            cpass.set_bind_group(3, &scene_bg, &[]);
            cpass.dispatch_workgroups(
                (self.grid_width * self.grid_height * self.grid_depth).div_ceil(64),
                1,
                1,
            );
        }

        // 3. Build Grid
        {
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Fluid::BuildGrid"),
                timestamp_writes: self.span_writes(3, true, true),
            });
            cpass.set_pipeline(&self.build_grid_pipeline);
            cpass.set_bind_group(0, global_bg, &[]);
            cpass.set_bind_group(1, particles_bg, &[]);
            cpass.set_bind_group(2, secondary_bg, &[]);
            cpass.set_bind_group(3, &scene_bg, &[]);
            cpass.dispatch_workgroups(particle_workgroups, 1, 1);
        }

        // 3. PBD Iterations (timing span 4 brackets the whole loop:
        //    begin on the first lambda pass, end on the last delta pass)
        for iter in 0..self.iterations {
            {
                let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("Fluid::Lambda"),
                    timestamp_writes: self.span_writes(4, iter == 0, false),
                });
                cpass.set_pipeline(&self.lambda_pipeline);
                cpass.set_bind_group(0, global_bg, &[]);
                cpass.set_bind_group(1, particles_bg, &[]);
                cpass.set_bind_group(2, secondary_bg, &[]);
                cpass.set_bind_group(3, &scene_bg, &[]);
                cpass.dispatch_workgroups(particle_workgroups, 1, 1);
            }
            {
                let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("Fluid::DeltaPos"),
                    timestamp_writes: self.span_writes(4, false, iter + 1 == self.iterations),
                });
                cpass.set_pipeline(&self.delta_pos_pipeline);
                cpass.set_bind_group(0, global_bg, &[]);
                cpass.set_bind_group(1, particles_bg, &[]);
                cpass.set_bind_group(2, secondary_bg, &[]);
                cpass.set_bind_group(3, &scene_bg, &[]);
                cpass.dispatch_workgroups(particle_workgroups, 1, 1);
            }
        }

        // 4. Integrate
        {
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Fluid::Integrate"),
                timestamp_writes: self.span_writes(5, true, true),
            });
            cpass.set_pipeline(&self.integrate_pipeline);
            cpass.set_bind_group(0, global_bg, &[]);
            cpass.set_bind_group(1, particles_bg, &[]);
            cpass.set_bind_group(2, secondary_bg, &[]);
            cpass.set_bind_group(3, &scene_bg, &[]);
            cpass.dispatch_workgroups(particle_workgroups, 1, 1);
        }

        // 5. Dye Mixing & Whitewater
        {
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Fluid::Dye&Whitewater"),
                timestamp_writes: self.span_writes(6, true, true),
            });
            cpass.set_bind_group(0, global_bg, &[]);
            cpass.set_bind_group(1, particles_bg, &[]);
            cpass.set_bind_group(2, secondary_bg, &[]);
            cpass.set_bind_group(3, &scene_bg, &[]);

            // Dye mixing
            cpass.set_pipeline(&self.mix_dye_pipeline);
            cpass.dispatch_workgroups(particle_workgroups, 1, 1);
        }
        // Resolve GPU timing queries (if enabled) into the timing staging
        // buffer; harvested by the blocking `read_gpu_timings`.
        if let Some(t) = &self.gpu_timing {
            let slots = (GPU_TIMING_SPANS.len() * 2) as u32;
            encoder.resolve_query_set(&t.query_set, 0..slots, &t.resolve_buffer, 0);
            encoder.copy_buffer_to_buffer(
                &t.resolve_buffer,
                0,
                &t.staging_buffer,
                0,
                slots as u64 * 8,
            );
        }

        // 6. Record the density-error copy for later (post-submit) readback.
        // We never map a buffer whose copy has not been submitted; the map is
        // requested by `pump_density_error_readback` at the start of a later
        // step() call, per the contract documented on this method.
        let staging_idx = self.frame_index % 2;
        if self.staging_state[staging_idx] == StagingState::Idle {
            encoder.copy_buffer_to_buffer(
                &self.density_error_buffer,
                0,
                &self.density_error_staging_buffers[staging_idx],
                0,
                4,
            );
            self.staging_state[staging_idx] = StagingState::CopyRecorded;
        }
        // else: the buffer is still in flight from an earlier frame (slow GPU
        // or skipped submits). Skip this frame's copy; adaptive feedback just
        // gets one frame staler.

        self.frame_index += 1;
    }

    /// Advance the density-error readback state machine (called at the start
    /// of every `step()`).
    ///
    /// - A buffer in `CopyRecorded` state had its copy submitted by the caller
    ///   (per the `step` contract), so it is now safe to request its mapping.
    /// - A buffer in `MapRequested` state whose mapping completed is read,
    ///   fed into the smoothed `AdaptiveIterations` controller, and unmapped.
    fn pump_density_error_readback(&mut self, device: &wgpu::Device) {
        use std::sync::atomic::Ordering;

        // Non-blocking poll so completed mappings get their callbacks fired.
        let _ = device.poll(wgpu::MaintainBase::Poll);

        for i in 0..2 {
            match self.staging_state[i] {
                StagingState::Idle => {}
                StagingState::CopyRecorded => {
                    // The copy's encoder was submitted before this step()
                    // call; request the mapping now.
                    self.staging_map_result[i].store(MAP_PENDING, Ordering::Release);
                    let result_flag = self.staging_map_result[i].clone();
                    self.density_error_staging_buffers[i].slice(..).map_async(
                        wgpu::MapMode::Read,
                        move |res| {
                            result_flag.store(
                                if res.is_ok() { MAP_OK } else { MAP_ERR },
                                Ordering::Release,
                            );
                        },
                    );
                    self.staging_state[i] = StagingState::MapRequested;
                    let _ = device.poll(wgpu::MaintainBase::Poll);
                }
                StagingState::MapRequested => {
                    match self.staging_map_result[i].load(Ordering::Acquire) {
                        MAP_OK => {
                            {
                                let data = self.density_error_staging_buffers[i]
                                    .slice(..)
                                    .get_mapped_range();
                                // Staging buffer is exactly 4 bytes.
                                let mut bytes = [0u8; 4];
                                bytes.copy_from_slice(&data[0..4]);
                                let error_scaled = u32::from_ne_bytes(bytes);
                                let avg_error =
                                    (error_scaled as f32 / 1000.0) / self.particle_count as f32;
                                // Smoothed controller; bounds configured at construction.
                                self.iterations = self.adaptive_iterations.update(avg_error);
                            }
                            self.density_error_staging_buffers[i].unmap();
                            self.staging_state[i] = StagingState::Idle;
                        }
                        MAP_ERR => {
                            // Mapping failed (e.g. device loss); drop the sample.
                            self.staging_state[i] = StagingState::Idle;
                        }
                        _ => {} // still pending — check again next step
                    }
                }
            }
        }
    }

    /// Returns the particle storage buffer.
    ///
    /// All compute kernels mutate the single particle buffer in place, so this
    /// is always the current simulation state (i.e. the state after the most
    /// recently *submitted* `step()` encoder executes).
    pub fn get_particle_buffer(&self) -> &wgpu::Buffer {
        &self.particle_buffer
    }

    pub fn secondary_particle_buffer(&self) -> &wgpu::Buffer {
        &self.secondary_particle_buffer
    }

    pub fn secondary_particle_count(&self) -> u32 {
        65536
    }

    // ==================== OPTIMIZATION API ====================

    /// Configure the fluid system with an optimization preset.
    /// This sets up workgroup sizes, iteration bounds, and quality settings.
    pub fn apply_optimization_preset(&mut self, preset: OptimizationPreset) {
        self.workgroup_config = preset.workgroups;
        self.adaptive_iterations = preset.adaptive_iterations;
        self.simulation_budget = preset.budget.clone();
        self.temporal_coherence.enabled = preset.temporal_coherence.enabled;
    }

    /// Configure for a specific GPU vendor for optimal workgroup sizes.
    pub fn set_gpu_vendor(&mut self, vendor: &str) {
        self.workgroup_config = match vendor.to_lowercase().as_str() {
            "nvidia" => WorkgroupConfig::nvidia(),
            "amd" => WorkgroupConfig::amd(),
            "intel" => WorkgroupConfig::intel(),
            _ => WorkgroupConfig::universal(),
        };
    }

    /// Set the frame time budget in milliseconds.
    /// The simulation will adjust quality to stay within this budget.
    pub fn set_time_budget_ms(&mut self, budget_ms: f32) {
        self.simulation_budget = SimulationBudget::new(budget_ms);
    }

    /// Enable or disable temporal coherence optimization.
    /// When enabled, resting particles are skipped to save GPU work.
    pub fn set_temporal_coherence(&mut self, enabled: bool) {
        self.temporal_coherence.enabled = enabled;
    }

    /// Set the number of solver iterations per frame.
    /// Higher values improve simulation accuracy but cost more GPU time.
    pub fn set_iterations(&mut self, iterations: u32) {
        self.iterations = iterations.clamp(1, 16); // Clamp to reasonable range
    }

    /// Get current iteration count.
    pub fn get_iterations(&self) -> u32 {
        self.iterations
    }

    /// Get current optimization statistics.
    pub fn get_optimization_stats(&self) -> &OptimizationStats {
        &self.optimization_stats
    }

    /// Optimized simulation step with budget-aware quality scaling.
    ///
    /// This method uses:
    /// - Adaptive iterations based on density error feedback
    /// - Frame budget tracking for quality scaling
    /// - Workgroup configuration optimized for the target GPU
    ///
    /// Returns the actual iteration count used this frame.
    pub fn step_with_budget(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        queue: &wgpu::Queue,
        dt: f32,
        frame_time_ms: f32,
    ) -> u32 {
        // Record last frame's time for budget tracking
        self.simulation_budget.record_frame(frame_time_ms);

        // Get quality-adjusted iteration count
        let quality = self.simulation_budget.quality();
        let recommended = self
            .simulation_budget
            .recommended_iterations(self.adaptive_iterations.max_iterations);

        // Blend between adaptive and budget-recommended iterations
        let under_budget = quality >= 0.8; // Consider under budget if quality is high
        let target_iterations = if under_budget {
            // Under budget: use adaptive (quality-focused)
            self.adaptive_iterations.current()
        } else {
            // Over budget: use recommended (performance-focused)
            recommended.min(self.adaptive_iterations.current())
        };

        // Set iterations for this frame (step()'s density-error feedback will
        // adaptively adjust self.iterations at the end of each frame)
        self.iterations = target_iterations;

        // Execute the simulation step
        self.step(device, encoder, queue, dt);

        // Update optimization stats
        self.optimization_stats = OptimizationStats {
            quality_level: quality,
            iterations: target_iterations,
            resting_particles: self.temporal_coherence.resting_particle_count() as u32,
            recommended_iterations: recommended,
            under_budget,
        };

        target_iterations
    }

    /// Flush any pending batch spawn operations.
    /// Call this before step() to spawn all queued particles efficiently.
    pub fn flush_batch_spawner(&mut self, queue: &wgpu::Queue) -> usize {
        let (positions, velocities, colors) = self.batch_spawner.flush();
        if positions.is_empty() {
            return 0;
        }

        // Convert to the format expected by spawn_particles
        let pos_refs: Vec<[f32; 3]> = positions;
        let vel_refs: Vec<[f32; 3]> = velocities;
        let color_refs: Vec<[f32; 4]> = colors;

        self.spawn_particles(queue, &pos_refs, &vel_refs, Some(&color_refs))
    }

    /// Queue a particle for batch spawning.
    /// Call flush_batch_spawner() before step() to actually spawn queued particles.
    pub fn queue_particle_spawn(
        &mut self,
        position: [f32; 3],
        velocity: [f32; 3],
        color: [f32; 4],
    ) -> bool {
        self.batch_spawner.queue(position, velocity, color)
    }

    /// Queue multiple particles for batch spawning.
    pub fn queue_particle_spawn_many(
        &mut self,
        positions: &[[f32; 3]],
        velocities: &[[f32; 3]],
        colors: &[[f32; 4]],
    ) {
        self.batch_spawner.queue_many(positions, velocities, colors);
    }

    /// Reset optimization state (call when simulation is reset).
    pub fn reset_optimization_state(&mut self) {
        self.adaptive_iterations.reset();
        self.simulation_budget.reset();
        self.temporal_coherence.reset();
        self.batch_spawner.clear();
        self.optimization_stats = OptimizationStats::default();
    }

    /// Get the optimal workgroup count for particle dispatch.
    pub fn optimal_particle_workgroups(&self) -> u32 {
        self.workgroup_config.particle_dispatch(self.particle_count)
    }

    /// Get the optimal workgroup count for grid dispatch.
    pub fn optimal_grid_workgroups(&self) -> u32 {
        let grid_size = self.grid_width * self.grid_height * self.grid_depth;
        self.workgroup_config.grid_dispatch(grid_size)
    }
}

// ============================================================================
// PRODUCTION OPTIMIZATION CONTROLLER
// ============================================================================

/// Production-ready optimization controller that manages all optimization
/// subsystems in a unified interface.
///
/// This controller coordinates:
/// - GPU vendor-specific workgroup tuning
/// - Adaptive iteration counts based on frame budget
/// - Particle streaming and LOD management
/// - Performance profiling and recommendations
/// - Automatic quality scaling
///
/// # Example
/// ```ignore
/// let mut controller = FluidOptimizationController::new();
/// controller.configure_for_gpu(&adapter_info);
/// controller.set_target_framerate(60.0);
///
/// // In game loop:
/// let context = controller.begin_frame();
/// let result = controller.step(&mut fluid_system, dt, camera_pos);
/// controller.end_frame(result);
///
/// // Periodically check recommendations:
/// for rec in controller.get_recommendations() {
///     println!("Optimization suggestion: {:?}", rec);
/// }
/// ```
#[derive(Debug)]
pub struct FluidOptimizationController {
    /// Current optimization preset
    preset: OptimizationPreset,

    /// Performance profiler for metrics tracking
    profiler: OptimizationProfiler,

    /// LOD manager (optional, for camera-based optimization)
    lod_manager: Option<OptimizedLodManager>,

    /// Particle streaming manager (optional, for large scenes)
    streaming_manager: Option<ParticleStreamingManager>,

    /// Current GPU vendor detected
    gpu_vendor: GpuVendor,

    /// Target frame time in milliseconds
    target_frame_time_ms: f32,

    /// Whether auto-tuning is enabled
    auto_tune_enabled: bool,

    /// Frames since last quality adjustment
    frames_since_adjustment: u32,

    /// Minimum frames between auto-adjustments
    adjustment_cooldown: u32,

    /// Current quality tier (0-3, where 0 is highest)
    quality_tier: u8,

    /// Maximum allowed quality tier (for user settings)
    max_quality_tier: u8,

    /// Accumulated frame time for averaging
    accumulated_frame_time: f32,

    /// Number of frames accumulated
    frame_count: u32,

    /// Whether to force full quality (disable optimizations)
    force_full_quality: bool,
}

impl Default for FluidOptimizationController {
    fn default() -> Self {
        Self::new()
    }
}

impl FluidOptimizationController {
    /// Create a new optimization controller with sensible defaults.
    pub fn new() -> Self {
        Self {
            preset: OptimizationPreset::performance(),
            profiler: OptimizationProfiler::new(300), // 5 seconds at 60fps
            lod_manager: None,
            streaming_manager: None,
            gpu_vendor: GpuVendor::Unknown,
            target_frame_time_ms: 16.67, // 60 FPS
            auto_tune_enabled: true,
            frames_since_adjustment: 0,
            adjustment_cooldown: 60, // 1 second at 60fps
            quality_tier: 1,         // Start at high quality
            max_quality_tier: 3,     // Allow down to low quality
            accumulated_frame_time: 0.0,
            frame_count: 0,
            force_full_quality: false,
        }
    }

    /// Create controller with specific quality preset.
    pub fn with_preset(preset: OptimizationPreset) -> Self {
        let quality_tier = match preset.budget.target_ms {
            t if t >= 8.0 => 0, // Essential - highest quality
            t if t >= 4.0 => 1, // High
            t if t >= 2.0 => 2, // Medium
            _ => 3,             // Low
        };

        Self {
            preset,
            quality_tier,
            ..Self::new()
        }
    }

    /// Configure controller based on GPU adapter information.
    ///
    /// Automatically detects vendor and applies optimal settings.
    pub fn configure_for_gpu(&mut self, adapter_name: &str) {
        // Detect GPU vendor from adapter name
        let name_lower = adapter_name.to_lowercase();

        self.gpu_vendor = if name_lower.contains("nvidia") || name_lower.contains("geforce") {
            GpuVendor::Nvidia
        } else if name_lower.contains("amd") || name_lower.contains("radeon") {
            GpuVendor::Amd
        } else if name_lower.contains("intel") {
            GpuVendor::Intel
        } else if name_lower.contains("apple")
            || name_lower.contains("m1")
            || name_lower.contains("m2")
            || name_lower.contains("m3")
        {
            GpuVendor::Apple
        } else {
            GpuVendor::Unknown
        };

        // Apply vendor-specific workgroup configuration
        self.preset.workgroups = WorkgroupConfig::for_gpu(self.gpu_vendor);

        // Adjust quality expectations based on vendor
        match self.gpu_vendor {
            GpuVendor::Nvidia => {
                // NVIDIA GPUs handle high workloads well
                self.max_quality_tier = 3;
            }
            GpuVendor::Amd => {
                // AMD GPUs are similar in capability
                self.max_quality_tier = 3;
            }
            GpuVendor::Intel => {
                // Integrated Intel may need quality limits
                self.max_quality_tier = 2;
                if self.quality_tier > 2 {
                    self.quality_tier = 2;
                }
            }
            GpuVendor::Apple => {
                // Apple Silicon is very capable
                self.max_quality_tier = 3;
            }
            GpuVendor::Unknown => {
                // Conservative for unknown hardware
                self.max_quality_tier = 2;
                if self.quality_tier > 2 {
                    self.quality_tier = 2;
                }
            }
        }
    }

    /// Get the detected GPU vendor.
    pub fn gpu_vendor(&self) -> GpuVendor {
        self.gpu_vendor
    }

    /// Set target framerate (controller will aim for this).
    pub fn set_target_framerate(&mut self, fps: f32) {
        self.target_frame_time_ms = 1000.0 / fps.max(1.0);
    }

    /// Get target frame time in milliseconds.
    pub fn target_frame_time_ms(&self) -> f32 {
        self.target_frame_time_ms
    }

    /// Enable or disable automatic quality tuning.
    pub fn set_auto_tune(&mut self, enabled: bool) {
        self.auto_tune_enabled = enabled;
    }

    /// Check if auto-tuning is enabled.
    pub fn auto_tune_enabled(&self) -> bool {
        self.auto_tune_enabled
    }

    /// Force full quality (disable all optimizations for screenshots, etc).
    pub fn set_force_full_quality(&mut self, force: bool) {
        self.force_full_quality = force;
    }

    /// Set the maximum quality tier allowed (0=Essential, 1=High, 2=Medium, 3=Low).
    pub fn set_max_quality_tier(&mut self, tier: u8) {
        self.max_quality_tier = tier.min(3);
        if self.quality_tier > self.max_quality_tier {
            self.quality_tier = self.max_quality_tier;
        }
    }

    /// Directly set the current quality tier (0=Essential, 1=High, 2=Medium, 3=Low).
    ///
    /// This immediately changes the quality tier used for optimization decisions.
    /// The tier is clamped to the range [0, max_quality_tier].
    pub fn set_quality_tier(&mut self, tier: u8) {
        self.quality_tier = tier.min(self.max_quality_tier).min(3);
    }

    /// Get the current quality tier.
    pub fn quality_tier(&self) -> u8 {
        self.quality_tier
    }

    /// Get a human-readable quality tier name.
    pub fn quality_tier_name(&self) -> &'static str {
        match self.quality_tier {
            0 => "Essential",
            1 => "High",
            2 => "Medium",
            _ => "Low",
        }
    }

    /// Enable LOD management for camera-distance based optimization.
    pub fn enable_lod(&mut self, camera_position: [f32; 3]) {
        let config = self.lod_config_for_tier(self.quality_tier);
        self.lod_manager = Some(OptimizedLodManager::with_camera_position(
            config,
            camera_position,
        ));
    }

    /// Disable LOD management.
    pub fn disable_lod(&mut self) {
        self.lod_manager = None;
    }

    /// Enable particle streaming for large scenes.
    pub fn enable_streaming(&mut self, particle_budget: usize) {
        self.streaming_manager = Some(ParticleStreamingManager::with_budget(particle_budget));
    }

    /// Disable particle streaming.
    pub fn disable_streaming(&mut self) {
        self.streaming_manager = None;
    }

    /// Get the current optimization preset.
    pub fn preset(&self) -> &OptimizationPreset {
        &self.preset
    }

    /// Get the profiler for metrics access.
    pub fn profiler(&self) -> &OptimizationProfiler {
        &self.profiler
    }

    /// Get mutable profiler access.
    pub fn profiler_mut(&mut self) -> &mut OptimizationProfiler {
        &mut self.profiler
    }

    /// Get the LOD manager if enabled.
    pub fn lod_manager(&self) -> Option<&OptimizedLodManager> {
        self.lod_manager.as_ref()
    }

    /// Get the streaming manager if enabled.
    pub fn streaming_manager(&self) -> Option<&ParticleStreamingManager> {
        self.streaming_manager.as_ref()
    }

    /// Record frame timing and update auto-tuning.
    ///
    /// Call this after each simulation step with the actual frame time.
    pub fn record_frame(&mut self, frame_time_ms: f32) {
        // Update profiler - convert ms to us
        let time_us = (frame_time_ms * 1000.0) as u64;
        let mut metrics = OptimizationMetrics::new();
        metrics.record_frame(time_us, 0); // 0 iterations since we don't know
        self.profiler.record(metrics);

        // Accumulate for averaging
        self.accumulated_frame_time += frame_time_ms;
        self.frame_count += 1;
        self.frames_since_adjustment += 1;

        // Auto-tune if enabled and cooldown passed
        if self.auto_tune_enabled && self.frames_since_adjustment >= self.adjustment_cooldown {
            self.auto_adjust_quality();
        }
    }

    /// Get optimization recommendations based on recent performance.
    pub fn get_recommendations(&self) -> Vec<OptimizationRecommendation> {
        if let Some(metrics) = self.profiler.latest() {
            analyze_metrics(
                metrics,
                self.target_frame_time_ms,
                self.preset.temporal_coherence.enabled,
                self.preset.use_morton_sorting,
            )
        } else {
            Vec::new()
        }
    }

    /// Get a summary of current optimization state.
    pub fn status(&self) -> OptimizationControllerStatus {
        let avg_frame_time = if self.frame_count > 0 {
            self.accumulated_frame_time / self.frame_count as f32
        } else {
            0.0
        };

        let fps = if avg_frame_time > 0.0 {
            1000.0 / avg_frame_time
        } else {
            0.0
        };

        let (p1, p50, p99) = self.profiler.frame_time_percentiles();

        OptimizationControllerStatus {
            gpu_vendor: self.gpu_vendor,
            quality_tier: self.quality_tier,
            quality_tier_name: self.quality_tier_name().to_string(),
            target_frame_time_ms: self.target_frame_time_ms,
            avg_frame_time_ms: avg_frame_time,
            current_fps: fps,
            frame_time_p1: p1,
            frame_time_p50: p50,
            frame_time_p99: p99,
            auto_tune_enabled: self.auto_tune_enabled,
            force_full_quality: self.force_full_quality,
            lod_enabled: self.lod_manager.is_some(),
            streaming_enabled: self.streaming_manager.is_some(),
            frames_recorded: self.frame_count,
            within_budget: self.is_within_budget(),
            recommendations: self.get_recommendations(),
        }
    }

    /// Reset all accumulated metrics and start fresh.
    pub fn reset_metrics(&mut self) {
        self.profiler.clear();
        self.accumulated_frame_time = 0.0;
        self.frame_count = 0;
        self.frames_since_adjustment = 0;
    }

    // ========================================================================
    // PRIVATE METHODS
    // ========================================================================

    fn auto_adjust_quality(&mut self) {
        if self.frame_count < 30 {
            return; // Need enough samples
        }

        let avg_frame_time = self.accumulated_frame_time / self.frame_count as f32;
        let target = self.target_frame_time_ms;

        // Calculate headroom (positive = under budget, negative = over budget)
        let headroom_ratio = (target - avg_frame_time) / target;

        if headroom_ratio < -0.1 {
            // More than 10% over budget - decrease quality
            self.decrease_quality();
        } else if headroom_ratio > 0.3 && self.quality_tier > 0 {
            // More than 30% under budget and not at max quality - increase quality
            self.increase_quality();
        }

        // Reset accumulators
        self.accumulated_frame_time = 0.0;
        self.frame_count = 0;
        self.frames_since_adjustment = 0;
    }

    fn increase_quality(&mut self) {
        if self.quality_tier > 0 {
            self.quality_tier -= 1;
            self.apply_quality_tier();
        }
    }

    fn decrease_quality(&mut self) {
        if self.quality_tier < self.max_quality_tier {
            self.quality_tier += 1;
            self.apply_quality_tier();
        }
    }

    fn apply_quality_tier(&mut self) {
        self.preset = self.preset_for_tier(self.quality_tier);

        // Update LOD if enabled - compute config and camera position first to avoid borrow conflicts
        if self.lod_manager.is_some() {
            let lod_config = self.lod_config_for_tier(self.quality_tier);
            let camera_pos = self
                .lod_manager
                .as_ref()
                .expect("lod_manager must be initialized")
                .camera_position();
            self.lod_manager = Some(OptimizedLodManager::with_camera_position(
                lod_config, camera_pos,
            ));
        }
    }

    fn preset_for_tier(&self, tier: u8) -> OptimizationPreset {
        let mut preset = match tier {
            0 => OptimizationPreset::quality(),
            1 => OptimizationPreset::balanced(),
            2 => OptimizationPreset::performance(),
            _ => {
                // Low quality - more aggressive than performance
                let mut p = OptimizationPreset::performance();
                p.budget.target_ms = 1.0;
                p.adaptive_iterations = AdaptiveIterations::new(1, 2);
                p.temporal_coherence.velocity_threshold = 0.02;
                p
            }
        };

        // Preserve GPU vendor settings
        preset.workgroups = WorkgroupConfig::for_gpu(self.gpu_vendor);
        preset
    }

    fn lod_config_for_tier(&self, tier: u8) -> OptimizedLodConfig {
        match tier {
            0 => OptimizedLodConfig::from_preset(&OptimizationPreset::quality()),
            1 => OptimizedLodConfig::from_preset(&OptimizationPreset::balanced()),
            2 => OptimizedLodConfig::from_preset(&OptimizationPreset::performance()),
            _ => {
                let mut config =
                    OptimizedLodConfig::from_preset(&OptimizationPreset::performance());
                // Allow very low particle counts for lowest tier
                config.particle_factors = [0.5, 0.25, 0.1, 0.05];
                config
            }
        }
    }
}

/// Status report from the optimization controller.
#[derive(Debug, Clone)]
pub struct OptimizationControllerStatus {
    /// Detected GPU vendor
    pub gpu_vendor: GpuVendor,

    /// Current quality tier (0=Essential, 1=High, 2=Medium, 3=Low)
    pub quality_tier: u8,

    /// Human-readable quality tier name
    pub quality_tier_name: String,

    /// Target frame time in milliseconds
    pub target_frame_time_ms: f32,

    /// Average frame time over measurement period
    pub avg_frame_time_ms: f32,

    /// Current FPS estimate
    pub current_fps: f32,

    /// 1st percentile frame time (best case)
    pub frame_time_p1: f32,

    /// 50th percentile frame time (median)
    pub frame_time_p50: f32,

    /// 99th percentile frame time (worst case)
    pub frame_time_p99: f32,

    /// Whether auto-tuning is enabled
    pub auto_tune_enabled: bool,

    /// Whether force full quality is enabled
    pub force_full_quality: bool,

    /// Whether LOD management is enabled
    pub lod_enabled: bool,

    /// Whether particle streaming is enabled
    pub streaming_enabled: bool,

    /// Number of frames recorded for metrics
    pub frames_recorded: u32,

    /// Whether currently within frame budget
    pub within_budget: bool,

    /// Current optimization recommendations
    pub recommendations: Vec<OptimizationRecommendation>,
}

impl std::fmt::Display for OptimizationControllerStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "=== Fluid Optimization Controller Status ===")?;
        writeln!(f, "GPU: {:?}", self.gpu_vendor)?;
        writeln!(
            f,
            "Quality: {} (tier {})",
            self.quality_tier_name, self.quality_tier
        )?;
        writeln!(
            f,
            "Target: {:.2}ms ({:.0} FPS)",
            self.target_frame_time_ms,
            1000.0 / self.target_frame_time_ms
        )?;
        writeln!(
            f,
            "Current: {:.2}ms ({:.1} FPS)",
            self.avg_frame_time_ms, self.current_fps
        )?;
        writeln!(
            f,
            "Percentiles: P1={:.2}ms P50={:.2}ms P99={:.2}ms",
            self.frame_time_p1, self.frame_time_p50, self.frame_time_p99
        )?;
        writeln!(
            f,
            "Auto-tune: {} | Force Quality: {}",
            self.auto_tune_enabled, self.force_full_quality
        )?;
        writeln!(
            f,
            "LOD: {} | Streaming: {}",
            self.lod_enabled, self.streaming_enabled
        )?;
        writeln!(f, "Frames Recorded: {}", self.frames_recorded)?;

        if !self.recommendations.is_empty() {
            writeln!(f, "Recommendations:")?;
            for rec in &self.recommendations {
                writeln!(f, "  - {:?}", rec)?;
            }
        }

        Ok(())
    }
}

// ============================================================================
// OPTIMIZATION RENDER CONTEXT
// ============================================================================

/// Context for a single frame of optimized fluid rendering.
///
/// Use this to coordinate optimization decisions across a frame:
/// - LOD calculation based on camera position
/// - Particle streaming decisions
/// - Quality scaling
///
/// # Example
/// ```ignore
/// let context = controller.create_render_context(camera_pos);
///
/// for fluid_body in &mut fluid_bodies {
///     if context.should_simulate(fluid_body.position) {
///         let quality = context.quality_for_position(fluid_body.position);
///         fluid_body.simulate_with_quality(quality, dt);
///     }
/// }
/// ```
#[derive(Debug, Clone)]
pub struct FluidRenderContext {
    /// Camera position for LOD calculations
    pub camera_position: [f32; 3],

    /// Current quality tier
    pub quality_tier: u8,

    /// Whether force full quality is active
    pub force_full_quality: bool,

    /// Maximum simulation distance (objects beyond this are culled)
    pub max_simulation_distance: f32,

    /// Distance thresholds for LOD levels
    pub lod_distances: [f32; 4],

    /// Particle factor for each LOD level
    pub lod_particle_factors: [f32; 4],

    /// Current frame's simulation budget (in particles)
    pub particle_budget: usize,
}

impl FluidRenderContext {
    /// Calculate distance from camera to a world position.
    pub fn distance_to(&self, position: [f32; 3]) -> f32 {
        let dx = position[0] - self.camera_position[0];
        let dy = position[1] - self.camera_position[1];
        let dz = position[2] - self.camera_position[2];
        (dx * dx + dy * dy + dz * dz).sqrt()
    }

    /// Check if a fluid body at the given position should be simulated.
    pub fn should_simulate(&self, position: [f32; 3]) -> bool {
        let distance = self.distance_to(position);
        distance <= self.max_simulation_distance
    }

    /// Get the LOD level for a position (0=highest detail, 3=lowest).
    pub fn lod_for_position(&self, position: [f32; 3]) -> u8 {
        if self.force_full_quality {
            return 0;
        }

        let distance = self.distance_to(position);
        for (i, &threshold) in self.lod_distances.iter().enumerate() {
            if distance <= threshold {
                return i as u8;
            }
        }
        3 // Furthest LOD
    }

    /// Get the particle factor for a position (1.0=full, <1.0=reduced).
    pub fn particle_factor_for_position(&self, position: [f32; 3]) -> f32 {
        if self.force_full_quality {
            return 1.0;
        }

        let lod = self.lod_for_position(position) as usize;
        self.lod_particle_factors[lod.min(3)]
    }

    /// Get the effective quality tier for a position.
    ///
    /// Combines global quality tier with distance-based LOD.
    pub fn quality_for_position(&self, position: [f32; 3]) -> u8 {
        if self.force_full_quality {
            return 0;
        }

        let lod = self.lod_for_position(position);
        // Combine quality tier and LOD (higher = lower quality)
        (self.quality_tier + lod / 2).min(3)
    }
}

impl FluidOptimizationController {
    /// Create a render context for the current frame.
    ///
    /// Use this at the start of each frame to get consistent optimization
    /// decisions for all fluid bodies.
    pub fn create_render_context(&self, camera_position: [f32; 3]) -> FluidRenderContext {
        let base_distances = match self.quality_tier {
            0 => [50.0, 100.0, 200.0, 500.0],
            1 => [30.0, 75.0, 150.0, 300.0],
            2 => [20.0, 50.0, 100.0, 200.0],
            _ => [15.0, 35.0, 75.0, 150.0],
        };

        let particle_factors = match self.quality_tier {
            0 => [1.0, 0.9, 0.7, 0.5],
            1 => [1.0, 0.75, 0.5, 0.25],
            2 => [0.75, 0.5, 0.25, 0.1],
            _ => [0.5, 0.25, 0.1, 0.05],
        };

        FluidRenderContext {
            camera_position,
            quality_tier: self.quality_tier,
            force_full_quality: self.force_full_quality,
            max_simulation_distance: base_distances[3] * 1.5,
            lod_distances: base_distances,
            lod_particle_factors: particle_factors,
            particle_budget: self
                .streaming_manager
                .as_ref()
                .map(|s| s.particle_budget())
                .unwrap_or(100_000),
        }
    }

    // ==================== FLUIDSYSTEM INTEGRATION ====================

    /// Apply this controller's optimization settings to a FluidSystem.
    ///
    /// This synchronizes the controller's preset with the FluidSystem's
    /// internal optimization state. Call this when:
    /// - Initially setting up optimization
    /// - After changing quality tier
    /// - When switching GPU configurations
    pub fn apply_to_system(&self, system: &mut FluidSystem) {
        system.apply_optimization_preset(self.preset.clone());
    }

    /// Perform an optimized simulation step with full controller integration.
    ///
    /// This method:
    /// 1. Applies current optimization preset to the system
    /// 2. Gets the recommended iteration count based on frame budget
    /// 3. Executes the simulation step
    /// 4. Records timing for auto-tuning
    /// 5. Updates LOD and streaming if enabled
    ///
    /// Returns the frame result with timing and iteration info.
    pub fn step_system(
        &mut self,
        system: &mut FluidSystem,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        queue: &wgpu::Queue,
        dt: f32,
        camera_position: [f32; 3],
    ) -> OptimizedStepResult {
        use std::time::Instant;

        // Sync preset with system
        system.apply_optimization_preset(self.preset.clone());

        // Get recommended iterations
        let iterations = self.recommended_iterations();

        // Set iterations on the system
        system.set_iterations(iterations);

        // Execute simulation step with timing
        let start = Instant::now();
        system.step(device, encoder, queue, dt);
        let frame_time_us = start.elapsed().as_micros() as u64;
        let frame_time_ms = frame_time_us as f32 / 1000.0;

        // Record for auto-tuning
        self.record_frame(frame_time_ms);

        // Update LOD manager with camera position and get result
        let lod_result = self
            .lod_manager
            .as_mut()
            .map(|lod| lod.update_with_timing(camera_position, [0.0, 0.0, 0.0], frame_time_ms));

        // Update streaming manager if enabled
        if let Some(ref mut streaming) = self.streaming_manager {
            if let Some(ref result) = lod_result {
                streaming.update(result, system.particle_count);
            }
        }

        OptimizedStepResult {
            frame_time_ms,
            iterations_used: iterations,
            density_error: 0.0, // Not available without GPU readback
            quality_tier: self.quality_tier,
            within_budget: frame_time_ms <= self.target_frame_time_ms,
        }
    }

    /// Perform a simulation step using the budget-aware method.
    ///
    /// This uses FluidSystem's step_with_budget which automatically
    /// adjusts iterations based on frame time feedback.
    #[allow(clippy::too_many_arguments)]
    pub fn step_with_budget(
        &mut self,
        system: &mut FluidSystem,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        queue: &wgpu::Queue,
        dt: f32,
        last_frame_time_ms: f32,
        camera_position: [f32; 3],
    ) -> OptimizedStepResult {
        use std::time::Instant;

        // Sync preset with system
        system.apply_optimization_preset(self.preset.clone());

        // Execute simulation step with timing
        let start = Instant::now();
        let iterations = system.step_with_budget(device, encoder, queue, dt, last_frame_time_ms);
        let frame_time_ms = start.elapsed().as_secs_f32() * 1000.0;

        // Record for auto-tuning
        self.record_frame(frame_time_ms);

        // Update LOD manager with camera position and get result
        let lod_result = self
            .lod_manager
            .as_mut()
            .map(|lod| lod.update_with_timing(camera_position, [0.0, 0.0, 0.0], frame_time_ms));

        // Update streaming manager if enabled
        if let Some(ref mut streaming) = self.streaming_manager {
            if let Some(ref result) = lod_result {
                streaming.update(result, system.particle_count);
            }
        }

        OptimizedStepResult {
            frame_time_ms,
            iterations_used: iterations,
            density_error: 0.0,
            quality_tier: self.quality_tier,
            within_budget: frame_time_ms <= self.target_frame_time_ms,
        }
    }

    /// Prepare a FluidSystem for an optimized step without executing it.
    ///
    /// Use this when you want to execute the step yourself but still
    /// benefit from controller-managed settings.
    ///
    /// Returns recommended iterations for this frame.
    pub fn prepare_step(&mut self, system: &mut FluidSystem, camera_position: [f32; 3]) -> u32 {
        // Sync preset with system
        system.apply_optimization_preset(self.preset.clone());

        // Update LOD manager with camera position
        if let Some(ref mut lod) = self.lod_manager {
            lod.update(camera_position, [0.0, 0.0, 0.0]);
        }

        // Get and set recommended iterations
        let iterations = self.recommended_iterations();
        system.set_iterations(iterations);

        iterations
    }

    /// Record the result of a manually-executed step.
    ///
    /// Call this after executing a step yourself to update the
    /// controller's metrics and auto-tuning.
    pub fn record_step_result(&mut self, frame_time_ms: f32, camera_position: [f32; 3]) {
        self.record_frame(frame_time_ms);

        // Update LOD with actual frame time
        if let Some(ref mut lod) = self.lod_manager {
            lod.update(camera_position, [0.0, 0.0, 0.0]);
        }
    }

    /// Begin a frame with this controller's optimizations applied.
    ///
    /// Returns a `FrameGuard` that can be used to track frame time automatically.
    /// The guard records frame time when dropped.
    pub fn begin_frame(&mut self, camera_position: [f32; 3]) -> FluidFrameGuard<'_> {
        // Update LOD manager with camera position
        if let Some(ref mut lod) = self.lod_manager {
            lod.update(camera_position, [0.0, 0.0, 0.0]);
        }

        FluidFrameGuard {
            controller: self,
            start_time: std::time::Instant::now(),
            camera_position,
        }
    }

    /// Get the current iteration count recommendation.
    ///
    /// Based on frame budget and recent performance, returns the
    /// recommended number of solver iterations.
    pub fn recommended_iterations(&self) -> u32 {
        match self.quality_tier {
            0 => self.preset.adaptive_iterations.max_iterations,
            1 => {
                (self.preset.adaptive_iterations.min_iterations
                    + self.preset.adaptive_iterations.max_iterations)
                    / 2
            }
            2 => self.preset.adaptive_iterations.min_iterations + 1,
            _ => self.preset.adaptive_iterations.min_iterations,
        }
    }

    /// Check if the system is currently meeting performance targets.
    pub fn is_within_budget(&self) -> bool {
        if self.frame_count == 0 {
            return true;
        }
        let avg_frame_time = self.accumulated_frame_time / self.frame_count as f32;
        avg_frame_time <= self.target_frame_time_ms
    }

    /// Get the current performance headroom as a percentage.
    ///
    /// Returns positive values when under budget (room to increase quality)
    /// and negative values when over budget (need to decrease quality).
    pub fn budget_headroom(&self) -> f32 {
        if self.frame_count == 0 || self.target_frame_time_ms <= 0.0 {
            return 100.0;
        }
        let avg_frame_time = self.accumulated_frame_time / self.frame_count as f32;
        ((self.target_frame_time_ms - avg_frame_time) / self.target_frame_time_ms) * 100.0
    }
}

/// Result of an optimized simulation step.
#[derive(Debug, Clone, Copy)]
pub struct OptimizedStepResult {
    /// Frame time in milliseconds
    pub frame_time_ms: f32,
    /// Number of solver iterations used
    pub iterations_used: u32,
    /// Average density error after solving
    pub density_error: f32,
    /// Quality tier at time of simulation
    pub quality_tier: u8,
    /// Whether the frame stayed within budget
    pub within_budget: bool,
}

impl OptimizedStepResult {
    /// Check if this frame suggests increasing quality.
    pub fn suggests_quality_increase(&self) -> bool {
        self.within_budget && self.density_error < 0.01
    }

    /// Check if this frame suggests decreasing quality.
    pub fn suggests_quality_decrease(&self) -> bool {
        !self.within_budget
    }
}

/// RAII guard for automatic frame timing.
///
/// Created by `FluidOptimizationController::begin_frame()`.
/// Records frame time when dropped.
pub struct FluidFrameGuard<'a> {
    controller: &'a mut FluidOptimizationController,
    start_time: std::time::Instant,
    camera_position: [f32; 3],
}

impl<'a> FluidFrameGuard<'a> {
    /// Get the elapsed time so far for this frame.
    pub fn elapsed_ms(&self) -> f32 {
        self.start_time.elapsed().as_secs_f32() * 1000.0
    }

    /// Get the camera position for this frame.
    pub fn camera_position(&self) -> [f32; 3] {
        self.camera_position
    }

    /// Get current quality tier.
    pub fn quality_tier(&self) -> u8 {
        self.controller.quality_tier
    }

    /// Get the render context for this frame.
    pub fn render_context(&self) -> FluidRenderContext {
        self.controller.create_render_context(self.camera_position)
    }

    /// End the frame manually and get the recorded frame time.
    pub fn finish(self) -> f32 {
        // Drop will handle recording
        self.elapsed_ms()
    }
}

impl<'a> Drop for FluidFrameGuard<'a> {
    fn drop(&mut self) {
        let frame_time_ms = self.elapsed_ms();
        self.controller.record_frame(frame_time_ms);

        // Update LOD with actual frame time
        if let Some(ref mut lod) = self.controller.lod_manager {
            lod.update(self.camera_position, [0.0, 0.0, 0.0]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_particle_default_values() {
        let particle = Particle {
            position: [0.0, 0.0, 0.0, 1.0],
            velocity: [0.0, 0.0, 0.0, 0.0],
            predicted_position: [0.0, 0.0, 0.0, 1.0],
            lambda: 0.0,
            density: 0.0,
            phase: 0,
            temperature: 293.0,
            color: [0.0; 4],
        };
        assert_eq!(particle.position[3], 1.0);
        assert_eq!(particle.density, 0.0);
    }

    #[test]
    fn test_particle_with_velocity() {
        let particle = Particle {
            position: [1.0, 2.0, 3.0, 1.0],
            velocity: [0.5, -1.0, 0.0, 0.0],
            predicted_position: [1.0, 1.0, 3.0, 1.0],
            lambda: 1.0,
            density: 1.0,
            phase: 0,
            temperature: 293.0,
            color: [1.0, 0.0, 0.0, 1.0],
        };
        assert_eq!(particle.velocity[1], -1.0);
        assert_eq!(particle.lambda, 1.0);
        assert_eq!(particle.density, 1.0);
    }

    #[test]
    fn test_particle_size() {
        // Ensure Particle is exactly the size we expect for GPU alignment
        // 3 * vec4 (pos, vel, pred) + 1 * vec4 (lambda, density, pad) + 1 * vec4 (color)
        // = 80 bytes
        assert_eq!(std::mem::size_of::<Particle>(), 80);
    }

    #[test]
    fn test_sim_params_default() {
        let _params = SimParams {
            smoothing_radius: 1.0,
            target_density: 1.0,
            viscosity: 1.0,
            surface_tension: 1.0,
            gravity: -9.81,
            dt: 0.016,
            particle_count: 100,
            grid_width: 10,
            grid_height: 10,
            grid_depth: 10,
            cell_size: 1.0,
            object_count: 0,
            _pad0: 0.0,
            _pad1: 0.0,
            _pad2: 0.0,
            _pad3: 0.0,
        };
    }

    #[test]
    fn test_sim_params_size() {
        // SimParams should be 64 bytes (16 * 4 bytes)
        assert_eq!(std::mem::size_of::<SimParams>(), 64);
    }

    #[test]
    fn test_sim_params_grid_configuration() {
        let params = SimParams {
            smoothing_radius: 1.0,
            target_density: 1.0,
            viscosity: 1.0,
            surface_tension: 1.0,
            gravity: -9.81,
            dt: 0.016,
            particle_count: 100,
            grid_width: 64,
            grid_height: 32,
            grid_depth: 16,
            cell_size: 1.0,
            object_count: 0,
            _pad0: 0.0,
            _pad1: 0.0,
            _pad2: 0.0,
            _pad3: 0.0,
        };
        assert_eq!(params.grid_width, 64);
    }

    #[test]
    fn test_particle_bytemuck_cast() {
        let particles = vec![
            Particle {
                position: [1.0, 2.0, 3.0, 1.0],
                velocity: [0.0, 0.0, 0.0, 0.0],
                predicted_position: [1.0, 2.0, 3.0, 1.0],
                lambda: 0.0,
                density: 1.0,
                phase: 0,
                temperature: 293.0,
                color: [0.0; 4],
            },
            Particle {
                position: [4.0, 5.0, 6.0, 1.0],
                velocity: [1.0, 1.0, 1.0, 0.0],
                predicted_position: [5.0, 6.0, 7.0, 1.0],
                lambda: 2.0,
                density: 2.0,
                phase: 0,
                temperature: 293.0,
                color: [1.0; 4],
            },
        ];

        // Test that bytemuck::cast_slice works
        let bytes: &[u8] = bytemuck::cast_slice(&particles);
        assert_eq!(bytes.len(), 2 * std::mem::size_of::<Particle>());

        // Cast back
        let recovered: &[Particle] = bytemuck::cast_slice(bytes);
        assert_eq!(recovered.len(), 2);
        assert_eq!(recovered[0].position[0], 1.0);
        assert_eq!(recovered[1].position[0], 4.0);
    }

    #[test]
    fn test_sim_params_bytemuck_cast() {
        let params = SimParams {
            smoothing_radius: 1.0,
            target_density: 10.0,
            viscosity: 50.0,
            surface_tension: 0.1,
            gravity: -9.8,
            dt: 0.016,
            particle_count: 1000,
            grid_width: 128,
            grid_height: 128,
            grid_depth: 128,
            cell_size: 1.2,
            object_count: 0,
            _pad0: 0.0,
            _pad1: 0.0,
            _pad2: 0.0,
            _pad3: 0.0,
        };

        let bytes: &[u8] = bytemuck::bytes_of(&params);
        assert_eq!(bytes.len(), std::mem::size_of::<SimParams>());

        let recovered: &SimParams = bytemuck::from_bytes(bytes);
        assert_eq!(recovered.particle_count, 1000);
        assert_eq!(recovered.grid_width, 128);
    }

    #[test]
    fn test_particle_initial_spacing() {
        // Test the spacing logic used in FluidSystem::new
        let particle_count = 1000u32;
        let spacing = 0.5;
        let size = (particle_count as f32).powf(1.0 / 3.0).ceil() as usize;

        // Size should be 10 or 11 depending on floating point (cube root of 1000 ≈ 10.0)
        // The actual value is 10.0 exactly, but floating point may give 10.000001, so ceil gives 11
        assert!(size == 10 || size == 11, "Expected 10 or 11, got {}", size);

        // Check positions for first particle
        let i = 0;
        let x = (i % size) as f32 * spacing - 5.0;
        let y = ((i / size) % size) as f32 * spacing + 2.0;
        let z = (i / (size * size)) as f32 * spacing - 5.0;
        assert_eq!(x, -5.0);
        assert_eq!(y, 2.0);
        assert_eq!(z, -5.0);
    }

    #[test]
    fn test_grid_indexing_consistency() {
        let cell_size = 1.2;
        let grid_width = 128u32;
        let grid_height = 128u32;

        let pos = [1.5, 2.5, 3.5];
        let gx = (pos[0] as f32 / cell_size).floor() as i32;
        let gy = (pos[1] as f32 / cell_size).floor() as i32;
        let gz = (pos[2] as f32 / cell_size).floor() as i32;

        assert_eq!(gx, 1);
        assert_eq!(gy, 2);
        assert_eq!(gz, 2);

        let cell_idx =
            (gx as u32) + (gy as u32) * grid_width + (gz as u32) * (grid_width * grid_height);

        assert_eq!(cell_idx, 1 + 2 * 128 + 2 * 128 * 128);
    }

    // ================== DynamicObject Tests ==================

    #[test]
    fn test_dynamic_object_size() {
        // DynamicObject should be 144 bytes:
        // transform: 4x4 f32 = 64 bytes
        // inv_transform: 4x4 f32 = 64 bytes
        // half_extents: 4 f32 = 16 bytes
        // Total = 144 bytes
        assert_eq!(std::mem::size_of::<DynamicObject>(), 144);
    }

    #[test]
    fn test_dynamic_object_identity() {
        let obj = DynamicObject {
            transform: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
            inv_transform: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
            half_extents: [1.0, 1.0, 1.0, 0.0], // Box type (w=0)
        };

        assert_eq!(obj.transform[0][0], 1.0);
        assert_eq!(obj.transform[3][3], 1.0);
        assert_eq!(obj.half_extents[3], 0.0); // Box type
    }

    #[test]
    fn test_dynamic_object_sphere() {
        let sphere = DynamicObject {
            transform: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [5.0, 10.0, 15.0, 1.0], // Position at (5, 10, 15)
            ],
            inv_transform: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [-5.0, -10.0, -15.0, 1.0],
            ],
            half_extents: [2.0, 2.0, 2.0, 1.0], // Sphere type (w=1), radius=2
        };

        assert_eq!(sphere.half_extents[3], 1.0); // Sphere type
        assert_eq!(sphere.transform[3][0], 5.0); // X position
    }

    #[test]
    fn test_dynamic_object_bytemuck_cast() {
        let obj = DynamicObject {
            transform: [[1.0; 4]; 4],
            inv_transform: [[2.0; 4]; 4],
            half_extents: [3.0, 4.0, 5.0, 0.0],
        };

        let bytes: &[u8] = bytemuck::bytes_of(&obj);
        assert_eq!(bytes.len(), std::mem::size_of::<DynamicObject>());

        let recovered: &DynamicObject = bytemuck::from_bytes(bytes);
        assert_eq!(recovered.transform[0][0], 1.0);
        assert_eq!(recovered.inv_transform[0][0], 2.0);
        assert_eq!(recovered.half_extents[0], 3.0);
    }

    #[test]
    fn test_dynamic_object_copy() {
        let obj = DynamicObject {
            transform: [[1.0; 4]; 4],
            inv_transform: [[1.0; 4]; 4],
            half_extents: [1.0; 4],
        };

        let copied = obj; // Copy trait
        assert_eq!(copied.transform[0][0], obj.transform[0][0]);
    }

    #[test]
    #[allow(clippy::clone_on_copy)]
    fn test_dynamic_object_clone() {
        let obj = DynamicObject {
            transform: [[5.0; 4]; 4],
            inv_transform: [[6.0; 4]; 4],
            half_extents: [1.0, 2.0, 3.0, 4.0],
        };

        let cloned = obj.clone();
        assert_eq!(cloned.half_extents, obj.half_extents);
    }

    // ================== SecondaryParticle Tests ==================

    #[test]
    fn test_secondary_particle_size() {
        // SecondaryParticle should be 48 bytes:
        // position: 4 f32 = 16 bytes
        // velocity: 4 f32 = 16 bytes
        // info: 4 f32 = 16 bytes
        // Total = 48 bytes
        assert_eq!(std::mem::size_of::<SecondaryParticle>(), 48);
    }

    #[test]
    fn test_secondary_particle_creation() {
        let particle = SecondaryParticle {
            position: [1.0, 2.0, 3.0, 1.0],
            velocity: [0.0, -1.0, 0.0, 0.0],
            info: [1.0, 0.0, 1.0, 0.5], // lifetime=1s, type=spray, alpha=1, scale=0.5
        };

        assert_eq!(particle.position[0], 1.0);
        assert_eq!(particle.velocity[1], -1.0);
        assert_eq!(particle.info[0], 1.0); // lifetime
        assert_eq!(particle.info[2], 1.0); // alpha
    }

    #[test]
    fn test_secondary_particle_types() {
        // Type 0: Spray
        let spray = SecondaryParticle {
            position: [0.0; 4],
            velocity: [0.0; 4],
            info: [1.0, 0.0, 1.0, 1.0], // type = 0 (spray)
        };
        assert_eq!(spray.info[1], 0.0);

        // Type 1: Foam
        let foam = SecondaryParticle {
            position: [0.0; 4],
            velocity: [0.0; 4],
            info: [2.0, 1.0, 0.8, 0.5], // type = 1 (foam)
        };
        assert_eq!(foam.info[1], 1.0);

        // Type 2: Bubbles
        let bubble = SecondaryParticle {
            position: [0.0; 4],
            velocity: [0.0; 4],
            info: [0.5, 2.0, 0.5, 0.25], // type = 2 (bubble)
        };
        assert_eq!(bubble.info[1], 2.0);
    }

    #[test]
    fn test_secondary_particle_bytemuck_cast() {
        let particles = vec![
            SecondaryParticle {
                position: [1.0, 2.0, 3.0, 1.0],
                velocity: [0.1, 0.2, 0.3, 0.0],
                info: [1.0, 0.0, 1.0, 1.0],
            },
            SecondaryParticle {
                position: [4.0, 5.0, 6.0, 1.0],
                velocity: [0.4, 0.5, 0.6, 0.0],
                info: [2.0, 1.0, 0.5, 0.5],
            },
        ];

        let bytes: &[u8] = bytemuck::cast_slice(&particles);
        assert_eq!(bytes.len(), 2 * std::mem::size_of::<SecondaryParticle>());

        let recovered: &[SecondaryParticle] = bytemuck::cast_slice(bytes);
        assert_eq!(recovered.len(), 2);
        assert_eq!(recovered[0].position[0], 1.0);
        assert_eq!(recovered[1].position[0], 4.0);
    }

    #[test]
    fn test_secondary_particle_copy() {
        let particle = SecondaryParticle {
            position: [1.0, 2.0, 3.0, 1.0],
            velocity: [0.0, -1.0, 0.0, 0.0],
            info: [1.0, 0.0, 1.0, 0.5],
        };

        let copied = particle; // Copy trait
        assert_eq!(copied.position, particle.position);
    }

    #[test]
    #[allow(clippy::clone_on_copy)]
    fn test_secondary_particle_clone() {
        let particle = SecondaryParticle {
            position: [10.0, 20.0, 30.0, 1.0],
            velocity: [1.0, 2.0, 3.0, 0.0],
            info: [5.0, 1.0, 0.8, 2.0],
        };

        let cloned = particle.clone();
        assert_eq!(cloned.info, particle.info);
    }

    // ================== Additional Particle Tests ==================

    #[test]
    fn test_particle_phase_values() {
        // Test different phase types
        let water = Particle {
            position: [0.0; 4],
            velocity: [0.0; 4],
            predicted_position: [0.0; 4],
            lambda: 0.0,
            density: 1.0,
            phase: 0, // Water
            temperature: 293.0,
            color: [0.2, 0.5, 0.8, 1.0],
        };
        assert_eq!(water.phase, 0);

        let oil = Particle {
            phase: 1, // Oil
            ..water
        };
        assert_eq!(oil.phase, 1);

        let custom = Particle {
            phase: 2, // Custom
            ..water
        };
        assert_eq!(custom.phase, 2);
    }

    #[test]
    fn test_particle_temperature() {
        // Test temperature handling
        let cold = Particle {
            position: [0.0; 4],
            velocity: [0.0; 4],
            predicted_position: [0.0; 4],
            lambda: 0.0,
            density: 1.0,
            phase: 0,
            temperature: 273.0, // 0°C
            color: [0.0; 4],
        };
        assert_eq!(cold.temperature, 273.0);

        let hot = Particle {
            temperature: 373.0, // 100°C
            ..cold
        };
        assert_eq!(hot.temperature, 373.0);
    }

    #[test]
    fn test_particle_color_channels() {
        let particle = Particle {
            position: [0.0; 4],
            velocity: [0.0; 4],
            predicted_position: [0.0; 4],
            lambda: 0.0,
            density: 1.0,
            phase: 0,
            temperature: 293.0,
            color: [1.0, 0.5, 0.25, 0.75], // RGBA
        };

        assert_eq!(particle.color[0], 1.0); // Red
        assert_eq!(particle.color[1], 0.5); // Green
        assert_eq!(particle.color[2], 0.25); // Blue
        assert_eq!(particle.color[3], 0.75); // Alpha
    }

    #[test]
    fn test_sim_params_dt_values() {
        let params_60fps = SimParams {
            smoothing_radius: 1.0,
            target_density: 1.0,
            viscosity: 1.0,
            surface_tension: 1.0,
            gravity: -9.81,
            dt: 1.0 / 60.0, // 60 FPS
            particle_count: 100,
            grid_width: 10,
            grid_height: 10,
            grid_depth: 10,
            cell_size: 1.0,
            object_count: 0,
            _pad0: 0.0,
            _pad1: 0.0,
            _pad2: 0.0,
            _pad3: 0.0,
        };

        // dt for 60 FPS should be approximately 0.0167
        assert!((params_60fps.dt - 0.0166666).abs() < 0.001);

        let params_144fps = SimParams {
            dt: 1.0 / 144.0, // 144 FPS
            ..params_60fps
        };

        // dt for 144 FPS should be approximately 0.00694
        assert!((params_144fps.dt - 0.00694).abs() < 0.001);
    }

    // ================== Despawn Region Tests ==================

    #[test]
    fn test_point_in_aabb() {
        // Helper to test AABB containment logic
        let min = [-5.0, -5.0, -5.0];
        let max = [5.0, 5.0, 5.0];

        let pos_inside = [0.0, 0.0, 0.0];
        let inside = pos_inside[0] >= min[0]
            && pos_inside[0] <= max[0]
            && pos_inside[1] >= min[1]
            && pos_inside[1] <= max[1]
            && pos_inside[2] >= min[2]
            && pos_inside[2] <= max[2];
        assert!(inside);

        let pos_outside = [10.0, 0.0, 0.0];
        let outside = pos_outside[0] >= min[0]
            && pos_outside[0] <= max[0]
            && pos_outside[1] >= min[1]
            && pos_outside[1] <= max[1]
            && pos_outside[2] >= min[2]
            && pos_outside[2] <= max[2];
        assert!(!outside);
    }

    #[test]
    fn test_aabb_validation() {
        // Test that min/max are correctly validated (swapped if needed)
        let min: [f32; 3] = [5.0, 5.0, 5.0];
        let max: [f32; 3] = [-5.0, -5.0, -5.0];

        // Should swap to correct orientation
        let valid_min = [min[0].min(max[0]), min[1].min(max[1]), min[2].min(max[2])];
        let valid_max = [min[0].max(max[0]), min[1].max(max[1]), min[2].max(max[2])];

        assert_eq!(valid_min, [-5.0, -5.0, -5.0]);
        assert_eq!(valid_max, [5.0, 5.0, 5.0]);
    }

    #[test]
    fn test_particle_position_cache_initial() {
        // Test that initial positions are correctly cached
        let spacing = 0.5;
        let size = (8_f32).powf(1.0 / 3.0).ceil() as usize; // 2

        let mut positions = Vec::new();
        for i in 0..8 {
            let x = (i % size) as f32 * spacing - 5.0;
            let y = ((i / size) % size) as f32 * spacing + 2.0;
            let z = (i / (size * size)) as f32 * spacing - 5.0;
            positions.push([x, y, z]);
        }

        assert_eq!(positions.len(), 8);
        // First particle
        assert_eq!(positions[0], [-5.0, 2.0, -5.0]);
    }

    #[test]
    fn test_despawn_region_processing_logic() {
        // Simulate the despawn processing logic without GPU
        let positions = vec![
            [0.0, 0.0, 0.0],   // Inside region
            [10.0, 0.0, 0.0],  // Outside region
            [2.0, 2.0, 2.0],   // Inside region
            [-10.0, 0.0, 0.0], // Outside region
        ];
        let mut active = vec![true; 4];
        let mut free_list = Vec::new();

        let regions = vec![
            ([-5.0, -5.0, -5.0], [5.0, 5.0, 5.0]), // Region around origin
        ];

        let mut despawned = 0;
        for idx in 0..positions.len() {
            if !active[idx] {
                continue;
            }

            let pos = positions[idx];
            let should_despawn = regions.iter().any(|(min, max)| {
                pos[0] >= min[0]
                    && pos[0] <= max[0]
                    && pos[1] >= min[1]
                    && pos[1] <= max[1]
                    && pos[2] >= min[2]
                    && pos[2] <= max[2]
            });

            if should_despawn {
                active[idx] = false;
                free_list.push(idx as u32);
                despawned += 1;
            }
        }

        assert_eq!(despawned, 2); // Particles 0 and 2
        assert_eq!(free_list, vec![0, 2]);
        assert_eq!(active, vec![false, true, false, true]);
    }

    #[test]
    fn test_despawn_multiple_regions() {
        // Test despawning with multiple overlapping regions
        let positions = vec![
            [0.0, 0.0, 0.0],   // Inside both regions
            [7.0, 0.0, 0.0],   // Inside second region only
            [-10.0, 0.0, 0.0], // Outside all regions
        ];
        let mut active = vec![true; 3];
        let mut free_list = Vec::new();

        let regions = vec![
            ([-5.0, -5.0, -5.0], [5.0, 5.0, 5.0]),
            ([0.0, -5.0, -5.0], [10.0, 5.0, 5.0]),
        ];

        let mut despawned = 0;
        for idx in 0..positions.len() {
            if !active[idx] {
                continue;
            }

            let pos = positions[idx];
            let should_despawn = regions.iter().any(|(min, max)| {
                pos[0] >= min[0]
                    && pos[0] <= max[0]
                    && pos[1] >= min[1]
                    && pos[1] <= max[1]
                    && pos[2] >= min[2]
                    && pos[2] <= max[2]
            });

            if should_despawn {
                active[idx] = false;
                free_list.push(idx as u32);
                despawned += 1;
            }
        }

        assert_eq!(despawned, 2); // Particles 0 and 1
        assert_eq!(free_list, vec![0, 1]);
    }

    #[test]
    fn test_despawn_boundary_conditions() {
        // Test particles exactly on AABB boundaries
        let positions = vec![
            [-5.0, 0.0, 0.0],   // On min.x boundary (inside)
            [5.0, 0.0, 0.0],    // On max.x boundary (inside)
            [-5.001, 0.0, 0.0], // Just outside min.x
            [5.001, 0.0, 0.0],  // Just outside max.x
        ];

        let min = [-5.0, -5.0, -5.0];
        let max = [5.0, 5.0, 5.0];

        let results: Vec<bool> = positions
            .iter()
            .map(|pos| {
                pos[0] >= min[0]
                    && pos[0] <= max[0]
                    && pos[1] >= min[1]
                    && pos[1] <= max[1]
                    && pos[2] >= min[2]
                    && pos[2] <= max[2]
            })
            .collect();

        assert!(results[0], "Particle on min.x boundary should be inside");
        assert!(results[1], "Particle on max.x boundary should be inside");
        assert!(!results[2], "Particle just outside min.x should be outside");
        assert!(!results[3], "Particle just outside max.x should be outside");
    }

    // ==================== Optimization Integration Tests ====================

    #[test]
    fn test_optimization_stats_default() {
        let stats = OptimizationStats::default();
        assert_eq!(stats.quality_level, 0.0);
        assert_eq!(stats.iterations, 0);
        assert_eq!(stats.resting_particles, 0);
        assert!(!stats.under_budget);
    }

    #[test]
    fn test_optimization_preset_application() {
        // Test that presets provide expected configurations
        let quality = OptimizationPreset::quality();
        assert_eq!(quality.adaptive_iterations.max_iterations, 8);
        assert!(quality.temporal_coherence.enabled);

        let performance = OptimizationPreset::performance();
        assert_eq!(performance.adaptive_iterations.max_iterations, 4);

        let balanced = OptimizationPreset::balanced();
        assert_eq!(balanced.adaptive_iterations.max_iterations, 6);
    }

    #[test]
    fn test_workgroup_config_vendors() {
        let nvidia = WorkgroupConfig::nvidia();
        assert_eq!(nvidia.particle_workgroup, 128);

        let amd = WorkgroupConfig::amd();
        assert_eq!(amd.particle_workgroup, 64);

        let intel = WorkgroupConfig::intel();
        assert_eq!(intel.particle_workgroup, 64);

        let universal = WorkgroupConfig::universal();
        assert_eq!(universal.particle_workgroup, 64);
    }

    #[test]
    fn test_adaptive_iterations_bounds() {
        let mut adaptive = AdaptiveIterations::new(2, 8);

        // Should start at middle
        assert!(adaptive.current() >= 2);
        assert!(adaptive.current() <= 8);

        // Update with low error should decrease
        let initial = adaptive.current();
        for _ in 0..10 {
            adaptive.update(0.001);
        }
        assert!(adaptive.current() <= initial);

        // Update with high error should increase
        adaptive.reset();
        let initial2 = adaptive.current();
        for _ in 0..10 {
            adaptive.update(0.1);
        }
        assert!(adaptive.current() >= initial2);
    }

    #[test]
    fn test_simulation_budget_tracking() {
        let mut budget = SimulationBudget::new(8.0);

        // Under budget - quality starts at 1.0
        budget.record_frame(5.0);
        assert!(budget.quality() >= 0.8); // Should stay high when under budget

        // Over budget - quality should decrease
        for _ in 0..10 {
            budget.record_frame(12.0);
        }
        assert!(budget.quality() < 1.0);
    }

    #[test]
    fn test_batch_spawner_queue_flush() {
        let mut spawner = BatchSpawner::new(100);

        // Queue some particles
        spawner.queue([0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [1.0, 1.0, 1.0, 1.0]);
        spawner.queue([1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 1.0, 0.0, 1.0]);

        assert_eq!(spawner.pending_count(), 2);
        assert!(!spawner.is_empty());

        // Flush
        let (pos, vel, col) = spawner.flush();
        assert_eq!(pos.len(), 2);
        assert_eq!(vel.len(), 2);
        assert_eq!(col.len(), 2);
        assert!(spawner.is_empty());
    }

    #[test]
    fn test_temporal_coherence_rest_detection() {
        let mut tc = TemporalCoherence::new(0.01, 3);
        tc.init(10);

        // Moving particle should always simulate
        assert!(tc.should_simulate(0, 1.0));
        assert!(tc.should_simulate(0, 1.0));

        // Resting particle should stop simulating after threshold
        assert!(tc.should_simulate(1, 0.001)); // Frame 1
        assert!(tc.should_simulate(1, 0.001)); // Frame 2
        assert!(!tc.should_simulate(1, 0.001)); // Frame 3 - now resting

        // Wake up when moving again
        assert!(tc.should_simulate(1, 1.0));
    }

    #[test]
    fn test_morton_code_spatial_ordering() {
        // Nearby positions should have nearby Morton codes
        let world_min = [0.0, 0.0, 0.0];
        let world_max = [20.0, 20.0, 20.0];
        let grid_resolution = 256;

        let code1 =
            MortonCode::from_position([0.0, 0.0, 0.0], world_min, world_max, grid_resolution);
        let code2 =
            MortonCode::from_position([0.5, 0.0, 0.0], world_min, world_max, grid_resolution);
        let code3 =
            MortonCode::from_position([10.0, 10.0, 10.0], world_min, world_max, grid_resolution);

        // code1 and code2 should be closer than code1 and code3
        let diff12 = if code1 > code2 {
            code1 - code2
        } else {
            code2 - code1
        };
        let diff13 = if code1 > code3 {
            code1 - code3
        } else {
            code3 - code1
        };

        assert!(
            diff12 < diff13,
            "Nearby positions should have similar Morton codes"
        );
    }
}

// ============================================================================
// OPTIMIZATION CONTROLLER TESTS
// ============================================================================

#[cfg(test)]
mod optimization_controller_tests {
    use super::*;

    #[test]
    fn test_controller_creation_default() {
        let controller = FluidOptimizationController::new();

        assert_eq!(controller.quality_tier(), 1); // High quality
        assert_eq!(controller.quality_tier_name(), "High");
        assert!((controller.target_frame_time_ms() - 16.67).abs() < 0.1); // 60 FPS
        assert!(controller.auto_tune_enabled());
        assert!(controller.lod_manager().is_none());
        assert!(controller.streaming_manager().is_none());
    }

    #[test]
    fn test_controller_with_preset() {
        let controller = FluidOptimizationController::with_preset(OptimizationPreset::quality());
        assert_eq!(controller.quality_tier(), 0); // Essential

        let controller = FluidOptimizationController::with_preset(OptimizationPreset::balanced());
        assert_eq!(controller.quality_tier(), 1); // High

        let controller =
            FluidOptimizationController::with_preset(OptimizationPreset::performance());
        assert_eq!(controller.quality_tier(), 2); // Medium
    }

    #[test]
    fn test_controller_gpu_detection_nvidia() {
        let mut controller = FluidOptimizationController::new();
        controller.configure_for_gpu("NVIDIA GeForce RTX 4090");

        assert!(matches!(controller.gpu_vendor(), GpuVendor::Nvidia));
        assert_eq!(controller.preset().workgroups.particle_workgroup, 128);
    }

    #[test]
    fn test_controller_gpu_detection_amd() {
        let mut controller = FluidOptimizationController::new();
        controller.configure_for_gpu("AMD Radeon RX 7900 XTX");

        assert!(matches!(controller.gpu_vendor(), GpuVendor::Amd));
        assert_eq!(controller.preset().workgroups.particle_workgroup, 64);
    }

    #[test]
    fn test_controller_gpu_detection_intel() {
        let mut controller = FluidOptimizationController::new();
        controller.configure_for_gpu("Intel Arc A770");

        assert!(matches!(controller.gpu_vendor(), GpuVendor::Intel));
        assert_eq!(controller.preset().workgroups.particle_workgroup, 64);
    }

    #[test]
    fn test_controller_gpu_detection_apple() {
        let mut controller = FluidOptimizationController::new();
        controller.configure_for_gpu("Apple M3 Max");

        assert!(matches!(controller.gpu_vendor(), GpuVendor::Apple));
        assert_eq!(controller.preset().workgroups.particle_workgroup, 256);
    }

    #[test]
    fn test_controller_gpu_detection_unknown() {
        let mut controller = FluidOptimizationController::new();
        controller.configure_for_gpu("Some Unknown GPU");

        assert!(matches!(controller.gpu_vendor(), GpuVendor::Unknown));
        // Unknown GPUs get limited quality
        assert!(controller.quality_tier() <= 2);
    }

    #[test]
    fn test_controller_target_framerate() {
        let mut controller = FluidOptimizationController::new();

        controller.set_target_framerate(30.0);
        assert!((controller.target_frame_time_ms() - 33.33).abs() < 0.1);

        controller.set_target_framerate(144.0);
        assert!((controller.target_frame_time_ms() - 6.944).abs() < 0.1);
    }

    #[test]
    fn test_controller_lod_enable_disable() {
        let mut controller = FluidOptimizationController::new();

        assert!(controller.lod_manager().is_none());

        controller.enable_lod([0.0, 10.0, 0.0]);
        assert!(controller.lod_manager().is_some());

        controller.disable_lod();
        assert!(controller.lod_manager().is_none());
    }

    #[test]
    fn test_controller_streaming_enable_disable() {
        let mut controller = FluidOptimizationController::new();

        assert!(controller.streaming_manager().is_none());

        controller.enable_streaming(50_000);
        assert!(controller.streaming_manager().is_some());
        assert_eq!(
            controller.streaming_manager().unwrap().particle_budget(),
            50_000
        );

        controller.disable_streaming();
        assert!(controller.streaming_manager().is_none());
    }

    #[test]
    fn test_controller_record_frame() {
        let mut controller = FluidOptimizationController::new();

        // Record some frames
        for _ in 0..10 {
            controller.record_frame(16.0);
        }

        let status = controller.status();
        assert_eq!(status.frames_recorded, 10);
        assert!((status.avg_frame_time_ms - 16.0).abs() < 0.1);
        assert!((status.current_fps - 62.5).abs() < 1.0);
    }

    #[test]
    fn test_controller_auto_tune_decrease_quality() {
        let mut controller = FluidOptimizationController::new();
        controller.set_target_framerate(60.0); // 16.67ms target

        let initial_tier = controller.quality_tier();

        // Simulate way over budget frames (25ms = 40 FPS)
        for _ in 0..70 {
            controller.record_frame(25.0);
        }

        // Should have decreased quality (higher tier number)
        assert!(controller.quality_tier() > initial_tier);
    }

    #[test]
    fn test_controller_auto_tune_increase_quality() {
        let mut controller = FluidOptimizationController::new();
        controller.set_target_framerate(60.0); // 16.67ms target

        // Start at low quality
        controller.set_max_quality_tier(3);
        for _ in 0..70 {
            controller.record_frame(25.0);
        }
        let low_tier = controller.quality_tier();

        // Now simulate very fast frames (5ms = 200 FPS)
        controller.reset_metrics();
        for _ in 0..70 {
            controller.record_frame(5.0);
        }

        // Should have increased quality (lower tier number or same if already at best)
        assert!(controller.quality_tier() <= low_tier);
    }

    #[test]
    fn test_controller_force_full_quality() {
        let mut controller = FluidOptimizationController::new();

        assert!(!controller.status().force_full_quality);

        controller.set_force_full_quality(true);
        assert!(controller.status().force_full_quality);

        controller.set_force_full_quality(false);
        assert!(!controller.status().force_full_quality);
    }

    #[test]
    fn test_controller_quality_tier_limits() {
        let mut controller = FluidOptimizationController::new();

        controller.set_max_quality_tier(1);

        // Tier should be clamped
        assert!(controller.quality_tier() <= 1);
    }

    #[test]
    fn test_controller_status_display() {
        let mut controller = FluidOptimizationController::new();
        controller.configure_for_gpu("NVIDIA GeForce RTX 4090");

        for _ in 0..5 {
            controller.record_frame(10.0);
        }

        let status = controller.status();
        let display = format!("{}", status);

        assert!(display.contains("Nvidia"));
        assert!(display.contains("High"));
        assert!(display.contains("FPS"));
    }

    #[test]
    fn test_controller_reset_metrics() {
        let mut controller = FluidOptimizationController::new();

        for _ in 0..20 {
            controller.record_frame(15.0);
        }

        assert!(controller.status().frames_recorded > 0);

        controller.reset_metrics();

        assert_eq!(controller.status().frames_recorded, 0);
    }

    #[test]
    fn test_render_context_creation() {
        let controller = FluidOptimizationController::new();
        let context = controller.create_render_context([0.0, 10.0, 0.0]);

        assert_eq!(context.camera_position, [0.0, 10.0, 0.0]);
        assert_eq!(context.quality_tier, 1);
        assert!(!context.force_full_quality);
        assert!(context.max_simulation_distance > 0.0);
    }

    #[test]
    fn test_render_context_distance_calculation() {
        let controller = FluidOptimizationController::new();
        let context = controller.create_render_context([0.0, 0.0, 0.0]);

        let dist = context.distance_to([3.0, 4.0, 0.0]);
        assert!((dist - 5.0).abs() < 0.001); // 3-4-5 triangle
    }

    #[test]
    fn test_render_context_should_simulate() {
        let controller = FluidOptimizationController::new();
        let context = controller.create_render_context([0.0, 0.0, 0.0]);

        // Close objects should simulate
        assert!(context.should_simulate([10.0, 0.0, 0.0]));

        // Far objects should not simulate
        assert!(!context.should_simulate([10000.0, 0.0, 0.0]));
    }

    #[test]
    fn test_render_context_lod_for_position() {
        let controller = FluidOptimizationController::new();
        let context = controller.create_render_context([0.0, 0.0, 0.0]);

        // Very close = LOD 0
        assert_eq!(context.lod_for_position([1.0, 0.0, 0.0]), 0);

        // Further away = higher LOD
        let lod_far = context.lod_for_position([500.0, 0.0, 0.0]);
        assert!(lod_far > 0);
    }

    #[test]
    fn test_render_context_particle_factor() {
        let controller = FluidOptimizationController::new();
        let context = controller.create_render_context([0.0, 0.0, 0.0]);

        // Close = high particle factor
        let factor_close = context.particle_factor_for_position([1.0, 0.0, 0.0]);
        assert!(factor_close >= 0.9);

        // Far = lower particle factor
        let factor_far = context.particle_factor_for_position([200.0, 0.0, 0.0]);
        assert!(factor_far < factor_close);
    }

    #[test]
    fn test_render_context_force_full_quality() {
        let mut controller = FluidOptimizationController::new();
        controller.set_force_full_quality(true);

        let context = controller.create_render_context([0.0, 0.0, 0.0]);

        // Force full quality should always return LOD 0 and factor 1.0
        assert_eq!(context.lod_for_position([1000.0, 0.0, 0.0]), 0);
        assert!((context.particle_factor_for_position([1000.0, 0.0, 0.0]) - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_render_context_quality_for_position() {
        let controller = FluidOptimizationController::new();
        let context = controller.create_render_context([0.0, 0.0, 0.0]);

        // Close objects get base quality tier
        let quality_close = context.quality_for_position([1.0, 0.0, 0.0]);

        // Far objects get degraded quality
        let quality_far = context.quality_for_position([300.0, 0.0, 0.0]);

        assert!(quality_far >= quality_close);
    }

    #[test]
    fn test_controller_profiler_access() {
        let mut controller = FluidOptimizationController::new();

        // Record some data
        controller.record_frame(15.0);

        // Access profiler
        let profiler = controller.profiler();
        assert!(profiler.latest().is_some());

        // Mutable access
        controller.profiler_mut().clear();
        assert!(controller.profiler().latest().is_none());
    }

    #[test]
    fn test_controller_recommendations() {
        let mut controller = FluidOptimizationController::new();
        controller.set_target_framerate(60.0);

        // Initially no recommendations (no data)
        assert!(controller.get_recommendations().is_empty());

        // Record fast frames
        for _ in 0..10 {
            controller.record_frame(5.0);
        }

        // Now we should have recommendations
        let recs = controller.get_recommendations();
        // May or may not have recommendations depending on metrics
        // At least the function should work without panic
        assert!(recs.len() <= 10); // Sanity check
    }

    #[test]
    fn test_controller_preset_access() {
        let controller = FluidOptimizationController::with_preset(OptimizationPreset::quality());
        let preset = controller.preset();

        assert!(preset.budget.target_ms >= 6.0);
        assert!(preset.adaptive_iterations.max_iterations >= 3);
    }

    #[test]
    fn test_status_struct_fields() {
        let mut controller = FluidOptimizationController::new();
        controller.configure_for_gpu("AMD Radeon");
        controller.enable_lod([0.0, 0.0, 0.0]);
        controller.enable_streaming(10000);
        controller.record_frame(12.0);

        let status = controller.status();

        assert!(matches!(status.gpu_vendor, GpuVendor::Amd));
        assert!(status.lod_enabled);
        assert!(status.streaming_enabled);
        assert!(status.frames_recorded > 0);
        assert!(status.avg_frame_time_ms > 0.0);
        assert!(status.current_fps > 0.0);
    }

    #[test]
    fn test_controller_auto_tune_disabled() {
        let mut controller = FluidOptimizationController::new();
        controller.set_auto_tune(false);

        let initial_tier = controller.quality_tier();

        // Even with terrible frame times, quality shouldn't change
        for _ in 0..100 {
            controller.record_frame(50.0);
        }

        assert_eq!(controller.quality_tier(), initial_tier);
    }

    // ==================== Phase 8 Integration Tests ====================

    #[test]
    fn test_recommended_iterations() {
        let mut controller = FluidOptimizationController::new();

        // High quality tier (1) should return moderate iterations
        assert_eq!(controller.quality_tier(), 1);
        let iters = controller.recommended_iterations();
        assert!(iters >= 2 && iters <= 8);

        // Force lower quality tier
        for _ in 0..100 {
            controller.record_frame(50.0); // Over budget
        }

        // Lower quality tier should return fewer iterations
        let iters_low = controller.recommended_iterations();
        assert!(iters_low <= iters);
    }

    #[test]
    fn test_is_within_budget() {
        let mut controller = FluidOptimizationController::new();
        controller.set_target_framerate(60.0); // 16.67ms target

        // Initially within budget (no frames recorded)
        assert!(controller.is_within_budget());

        // Record fast frames
        for _ in 0..10 {
            controller.record_frame(5.0);
        }
        assert!(controller.is_within_budget());

        // Record slow frames
        controller.reset_metrics();
        for _ in 0..10 {
            controller.record_frame(30.0);
        }
        assert!(!controller.is_within_budget());
    }

    #[test]
    fn test_budget_headroom() {
        let mut controller = FluidOptimizationController::new();
        controller.set_target_framerate(60.0); // 16.67ms target

        // Initially 100% headroom
        assert_eq!(controller.budget_headroom(), 100.0);

        // Record frame at 50% of budget
        for _ in 0..10 {
            controller.record_frame(8.33);
        }
        let headroom = controller.budget_headroom();
        assert!(headroom > 40.0 && headroom < 60.0); // ~50% headroom

        // Record frame over budget
        controller.reset_metrics();
        for _ in 0..10 {
            controller.record_frame(25.0);
        }
        let headroom = controller.budget_headroom();
        assert!(headroom < 0.0); // Negative headroom (over budget)
    }

    #[test]
    fn test_optimized_step_result() {
        let result = OptimizedStepResult {
            frame_time_ms: 5.0,
            iterations_used: 4,
            density_error: 0.005,
            quality_tier: 0,
            within_budget: true,
        };

        assert!(result.suggests_quality_increase());
        assert!(!result.suggests_quality_decrease());

        let over_budget = OptimizedStepResult {
            frame_time_ms: 25.0,
            iterations_used: 4,
            density_error: 0.005,
            quality_tier: 2,
            within_budget: false,
        };

        assert!(!over_budget.suggests_quality_increase());
        assert!(over_budget.suggests_quality_decrease());
    }

    #[test]
    fn test_frame_guard_elapsed() {
        let mut controller = FluidOptimizationController::new();
        let guard = controller.begin_frame([0.0, 0.0, 0.0]);

        // Should have some elapsed time
        std::thread::sleep(std::time::Duration::from_millis(1));
        assert!(guard.elapsed_ms() > 0.0);

        // Camera position should be preserved
        assert_eq!(guard.camera_position(), [0.0, 0.0, 0.0]);

        // Quality tier should match controller
        assert_eq!(guard.quality_tier(), 1);
    }

    #[test]
    fn test_frame_guard_render_context() {
        let mut controller = FluidOptimizationController::new();
        controller.enable_lod([5.0, 5.0, 5.0]);

        let expected_tier = controller.quality_tier();
        let guard = controller.begin_frame([10.0, 10.0, 10.0]);
        let context = guard.render_context();

        assert_eq!(context.camera_position, [10.0, 10.0, 10.0]);
        assert_eq!(context.quality_tier, expected_tier);
    }

    #[test]
    fn test_record_step_result() {
        let mut controller = FluidOptimizationController::new();
        controller.enable_lod([0.0, 0.0, 0.0]);

        // Record should update metrics
        controller.record_step_result(10.0, [5.0, 5.0, 5.0]);

        let status = controller.status();
        assert!(status.frames_recorded > 0);
        assert!(status.avg_frame_time_ms > 0.0);
    }

    #[test]
    fn test_set_get_iterations() {
        let mut adaptive = AdaptiveIterations::new(2, 8);

        // Current should be within bounds
        assert!(adaptive.current() >= 2);
        assert!(adaptive.current() <= 8);

        // Reset should work
        adaptive.reset();
        assert!(adaptive.current() >= 2);
    }

    #[test]
    fn test_set_quality_tier() {
        let mut controller = FluidOptimizationController::new();

        // Initial tier should be 1 (High quality - default for new controller)
        assert_eq!(controller.quality_tier(), 1);

        // Set tier to 2 (Medium)
        controller.set_quality_tier(2);
        assert_eq!(controller.quality_tier(), 2);

        // Set tier to 3 (Low)
        controller.set_quality_tier(3);
        assert_eq!(controller.quality_tier(), 3);

        // Setting tier beyond max should clamp to max_quality_tier
        controller.set_quality_tier(10);
        assert_eq!(controller.quality_tier(), 3); // Clamped to 3 (max)

        // Set tier back to 0 (Essential)
        controller.set_quality_tier(0);
        assert_eq!(controller.quality_tier(), 0);
    }

    #[test]
    fn test_status_within_budget_field() {
        let mut controller = FluidOptimizationController::new();
        controller.set_target_framerate(60.0); // 16.67ms budget

        // Initially within budget (no frames recorded)
        let status = controller.status();
        assert!(status.within_budget);

        // Record frames well under budget
        for _ in 0..10 {
            controller.record_frame(10.0); // 10ms < 16.67ms
        }
        let status = controller.status();
        assert!(status.within_budget);

        // Reset and record frames over budget
        controller.reset_metrics();
        for _ in 0..10 {
            controller.record_frame(25.0); // 25ms > 16.67ms
        }
        let status = controller.status();
        assert!(!status.within_budget);
    }
}
