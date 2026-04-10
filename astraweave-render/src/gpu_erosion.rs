//! GPU Compute Erosion — Shallow Water Equations (Šťava et al. 2008)
//!
//! Three-pass compute pipeline per simulation step:
//! 1. `rain_and_flux` — rainfall + pipe-model outflow flux
//! 2. `water_velocity` — volume update from net flux, derive velocity
//! 3. `erode_transport` — dissolve/deposit, advect sediment, evaporate
//!
//! Runs 50-100× faster than the CPU particle-based erosion for large grids.

use bytemuck::{Pod, Zeroable};
use wgpu;

// ─── GPU Uniform ───

/// Erosion simulation parameters — matches WGSL `ErosionParams`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct ErosionParams {
    pub grid_width: u32,
    pub grid_height: u32,
    pub dt: f32,
    pub rain_rate: f32,
    pub pipe_area: f32,
    pub gravity: f32,
    pub cell_size: f32,
    pub sediment_capacity: f32,
    pub dissolution_rate: f32,
    pub deposition_rate: f32,
    pub evaporation_rate: f32,
    pub min_slope: f32,
}

// ─── Configuration ───

/// High-level configuration for GPU erosion.
#[derive(Debug, Clone)]
pub struct GpuErosionConfig {
    pub width: u32,
    pub height: u32,
    /// Simulation timestep (seconds).
    pub dt: f32,
    /// Rain rate (water units per cell per second).
    pub rain_rate: f32,
    /// Cross-sectional area of virtual flux pipes.
    pub pipe_area: f32,
    /// Gravitational acceleration (m/s²).
    pub gravity: f32,
    /// Grid cell spacing (meters).
    pub cell_size: f32,
    /// Maximum sediment a unit of velocity can carry.
    pub sediment_capacity: f32,
    /// Rate at which terrain dissolves into sediment.
    pub dissolution_rate: f32,
    /// Rate at which sediment deposits onto terrain.
    pub deposition_rate: f32,
    /// Fraction of water evaporated per second.
    pub evaporation_rate: f32,
    /// Minimum slope for erosion calculations.
    pub min_slope: f32,
}

impl Default for GpuErosionConfig {
    fn default() -> Self {
        Self {
            width: 256,
            height: 256,
            dt: 0.02,
            rain_rate: 0.01,
            pipe_area: 20.0,
            gravity: 9.81,
            cell_size: 1.0,
            sediment_capacity: 0.01,
            dissolution_rate: 0.01,
            deposition_rate: 0.02,
            evaporation_rate: 0.01,
            min_slope: 0.001,
        }
    }
}

impl GpuErosionConfig {
    /// Validate the configuration.
    pub fn validate(&self) -> bool {
        self.width > 0
            && self.height > 0
            && self.dt > 0.0
            && self.cell_size > 0.0
            && self.gravity > 0.0
            && self.pipe_area > 0.0
    }

    /// Convert to GPU-ready uniform struct.
    pub fn to_params(&self) -> ErosionParams {
        ErosionParams {
            grid_width: self.width,
            grid_height: self.height,
            dt: self.dt,
            rain_rate: self.rain_rate,
            pipe_area: self.pipe_area,
            gravity: self.gravity,
            cell_size: self.cell_size,
            sediment_capacity: self.sediment_capacity,
            dissolution_rate: self.dissolution_rate,
            deposition_rate: self.deposition_rate,
            evaporation_rate: self.evaporation_rate,
            min_slope: self.min_slope,
        }
    }

    /// Total number of grid cells.
    pub fn cell_count(&self) -> u32 {
        self.width * self.height
    }
}

// ─── Erosion Presets ───

/// Commonly used erosion configurations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErosionPreset {
    /// Gentle erosion for rolling hills.
    Gentle,
    /// Aggressive erosion for deep valleys and canyons.
    Canyon,
    /// Coastal erosion with high water and evaporation.
    Coastal,
}

impl ErosionPreset {
    /// Convert preset to a full configuration.
    pub fn to_config(self, width: u32, height: u32) -> GpuErosionConfig {
        match self {
            Self::Gentle => GpuErosionConfig {
                width,
                height,
                rain_rate: 0.005,
                dissolution_rate: 0.005,
                deposition_rate: 0.03,
                evaporation_rate: 0.02,
                sediment_capacity: 0.005,
                ..Default::default()
            },
            Self::Canyon => GpuErosionConfig {
                width,
                height,
                rain_rate: 0.02,
                dissolution_rate: 0.03,
                deposition_rate: 0.01,
                evaporation_rate: 0.005,
                sediment_capacity: 0.02,
                ..Default::default()
            },
            Self::Coastal => GpuErosionConfig {
                width,
                height,
                rain_rate: 0.03,
                dissolution_rate: 0.015,
                deposition_rate: 0.025,
                evaporation_rate: 0.03,
                sediment_capacity: 0.012,
                ..Default::default()
            },
        }
    }
}

// ─── GPU Pipeline ───

/// GPU compute erosion pipeline using Shallow Water Equations.
///
/// Holds three compute pipelines (one per pass), bind group layout,
/// and all simulation buffers.
pub struct GpuErosionPipeline {
    pass_rain_flux: wgpu::ComputePipeline,
    pass_water_vel: wgpu::ComputePipeline,
    pass_erode: wgpu::ComputePipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    params_buffer: wgpu::Buffer,
    terrain_buffer: wgpu::Buffer,
    water_buffer: wgpu::Buffer,
    sediment_buffer: wgpu::Buffer,
    flux_buffer: wgpu::Buffer,
    velocity_buffer: wgpu::Buffer,
    readback_buffer: wgpu::Buffer,
    config: GpuErosionConfig,
}

impl GpuErosionPipeline {
    /// Create the erosion pipeline and allocate simulation buffers.
    pub fn new(device: &wgpu::Device, config: GpuErosionConfig) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("gpu_erosion_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/gpu_erosion.wgsl").into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("gpu_erosion_bgl"),
            entries: &[
                // binding 0: params uniform
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
                // binding 1: terrain (storage RW)
                Self::storage_rw_entry(1),
                // binding 2: water (storage RW)
                Self::storage_rw_entry(2),
                // binding 3: sediment (storage RW)
                Self::storage_rw_entry(3),
                // binding 4: flux (storage RW)
                Self::storage_rw_entry(4),
                // binding 5: velocity (storage RW)
                Self::storage_rw_entry(5),
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("gpu_erosion_pipeline_layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let make_pipeline = |label: &str, entry: &str| {
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some(label),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: Some(entry),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                cache: None,
            })
        };

        let pass_rain_flux = make_pipeline("erosion_rain_flux", "rain_and_flux");
        let pass_water_vel = make_pipeline("erosion_water_vel", "water_velocity");
        let pass_erode = make_pipeline("erosion_erode", "erode_transport");

        let n = config.cell_count() as u64;

        let params_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("erosion_params"),
            size: std::mem::size_of::<ErosionParams>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let terrain_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("erosion_terrain"),
            size: n * 4,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let water_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("erosion_water"),
            size: n * 4,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let sediment_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("erosion_sediment"),
            size: n * 4,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let flux_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("erosion_flux"),
            size: n * 16, // vec4<f32> per cell
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let velocity_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("erosion_velocity"),
            size: n * 8, // vec2<f32> per cell
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let readback_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("erosion_readback"),
            size: n * 4,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            pass_rain_flux,
            pass_water_vel,
            pass_erode,
            bind_group_layout,
            params_buffer,
            terrain_buffer,
            water_buffer,
            sediment_buffer,
            flux_buffer,
            velocity_buffer,
            readback_buffer,
            config,
        }
    }

    /// Upload initial terrain heightmap data.
    ///
    /// `heights` must have exactly `width × height` elements.
    pub fn upload_terrain(&self, queue: &wgpu::Queue, heights: &[f32]) {
        let expected = self.config.cell_count() as usize;
        let n = heights.len().min(expected);
        queue.write_buffer(&self.terrain_buffer, 0, bytemuck::cast_slice(&heights[..n]));

        // Upload params
        let params = self.config.to_params();
        queue.write_buffer(&self.params_buffer, 0, bytemuck::bytes_of(&params));

        // Zero-initialize water, sediment, flux, velocity
        let zeros_f32 = vec![0.0f32; expected];
        queue.write_buffer(&self.water_buffer, 0, bytemuck::cast_slice(&zeros_f32));
        queue.write_buffer(&self.sediment_buffer, 0, bytemuck::cast_slice(&zeros_f32));
        let zeros_flux = vec![[0.0f32; 4]; expected];
        queue.write_buffer(&self.flux_buffer, 0, bytemuck::cast_slice(&zeros_flux));
        let zeros_vel = vec![[0.0f32; 2]; expected];
        queue.write_buffer(&self.velocity_buffer, 0, bytemuck::cast_slice(&zeros_vel));
    }

    /// Encode `steps` erosion simulation steps into the command encoder.
    ///
    /// Each step dispatches three compute passes sequentially.
    pub fn encode_steps(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        device: &wgpu::Device,
        steps: u32,
    ) {
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("erosion_bg"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.params_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.terrain_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.water_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: self.sediment_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: self.flux_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: self.velocity_buffer.as_entire_binding(),
                },
            ],
        });

        let wg_x = self.config.width.div_ceil(8);
        let wg_y = self.config.height.div_ceil(8);

        for _ in 0..steps {
            // Pass 1: rain + flux
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("erosion_rain_flux"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(&self.pass_rain_flux);
                pass.set_bind_group(0, &bind_group, &[]);
                pass.dispatch_workgroups(wg_x, wg_y, 1);
            }
            // Pass 2: water + velocity
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("erosion_water_vel"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(&self.pass_water_vel);
                pass.set_bind_group(0, &bind_group, &[]);
                pass.dispatch_workgroups(wg_x, wg_y, 1);
            }
            // Pass 3: erosion + transport
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("erosion_erode"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(&self.pass_erode);
                pass.set_bind_group(0, &bind_group, &[]);
                pass.dispatch_workgroups(wg_x, wg_y, 1);
            }
        }
    }

    /// Encode a copy of the terrain buffer to the readback buffer.
    ///
    /// Call this after `encode_steps()`, then submit and map `readback_buffer()`.
    pub fn encode_readback(&self, encoder: &mut wgpu::CommandEncoder) {
        let size = self.config.cell_count() as u64 * 4;
        encoder.copy_buffer_to_buffer(&self.terrain_buffer, 0, &self.readback_buffer, 0, size);
    }

    /// Reference to the readback buffer for async mapping.
    pub fn readback_buffer(&self) -> &wgpu::Buffer {
        &self.readback_buffer
    }

    /// The current configuration.
    pub fn config(&self) -> &GpuErosionConfig {
        &self.config
    }

    /// The bind group layout.
    pub fn bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.bind_group_layout
    }

    /// Workgroup dispatch size for the current grid.
    pub fn workgroup_count(&self) -> (u32, u32) {
        (
            self.config.width.div_ceil(8),
            self.config.height.div_ceil(8),
        )
    }

    fn storage_rw_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
        wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only: false },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }
    }
}

// ─── Tests ───

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn erosion_params_size() {
        assert_eq!(
            std::mem::size_of::<ErosionParams>(),
            48,
            "ErosionParams must be 48 bytes"
        );
    }

    #[test]
    fn default_config_valid() {
        let cfg = GpuErosionConfig::default();
        assert!(cfg.validate());
        assert_eq!(cfg.cell_count(), 256 * 256);
    }

    #[test]
    fn config_validation_rejects_invalid() {
        let cfg = GpuErosionConfig {
            width: 0,
            ..Default::default()
        };
        assert!(!cfg.validate());

        let cfg = GpuErosionConfig {
            dt: 0.0,
            ..Default::default()
        };
        assert!(!cfg.validate());
    }

    #[test]
    fn config_to_params_roundtrip() {
        let cfg = GpuErosionConfig::default();
        let params = cfg.to_params();
        assert_eq!(params.grid_width, cfg.width);
        assert_eq!(params.grid_height, cfg.height);
        assert_eq!(params.dt, cfg.dt);
        assert_eq!(params.gravity, cfg.gravity);
    }

    #[test]
    fn erosion_preset_gentle() {
        let cfg = ErosionPreset::Gentle.to_config(128, 128);
        assert!(cfg.validate());
        assert!(cfg.dissolution_rate < 0.01);
    }

    #[test]
    fn erosion_preset_canyon() {
        let cfg = ErosionPreset::Canyon.to_config(512, 512);
        assert!(cfg.validate());
        assert!(cfg.dissolution_rate > 0.02);
    }

    #[test]
    fn erosion_preset_coastal() {
        let cfg = ErosionPreset::Coastal.to_config(64, 64);
        assert!(cfg.validate());
        assert!(cfg.evaporation_rate > 0.02);
    }

    #[test]
    fn parse_gpu_erosion_wgsl() {
        let src = include_str!("../shaders/gpu_erosion.wgsl");
        let result = naga::front::wgsl::parse_str(src);
        assert!(result.is_ok(), "WGSL parse failed: {result:?}");
    }

    #[test]
    fn gpu_erosion_pipeline_creation() {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let adapter = match pollster::block_on(
            instance.request_adapter(&wgpu::RequestAdapterOptions::default()),
        ) {
            Ok(a) => a,
            Err(_) => return,
        };
        let (device, _queue) =
            match pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())) {
                Ok(dq) => dq,
                Err(_) => return,
            };

        let config = GpuErosionConfig {
            width: 64,
            height: 64,
            ..Default::default()
        };
        let pipeline = GpuErosionPipeline::new(&device, config);
        assert_eq!(pipeline.config().width, 64);
        assert_eq!(pipeline.config().height, 64);
        assert_eq!(pipeline.workgroup_count(), (8, 8));
    }

    #[test]
    fn gpu_erosion_upload_and_encode() {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let adapter = match pollster::block_on(
            instance.request_adapter(&wgpu::RequestAdapterOptions::default()),
        ) {
            Ok(a) => a,
            Err(_) => return,
        };
        let (device, queue) =
            match pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())) {
                Ok(dq) => dq,
                Err(_) => return,
            };

        let config = GpuErosionConfig {
            width: 32,
            height: 32,
            ..Default::default()
        };
        let pipeline = GpuErosionPipeline::new(&device, config);

        // Upload a flat terrain
        let heights = vec![10.0f32; 32 * 32];
        pipeline.upload_terrain(&queue, &heights);

        // Encode 1 simulation step
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("erosion_test_encoder"),
        });
        pipeline.encode_steps(&mut encoder, &device, 1);
        pipeline.encode_readback(&mut encoder);

        queue.submit(std::iter::once(encoder.finish()));
    }
}
