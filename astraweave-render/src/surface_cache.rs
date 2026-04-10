//! Surface Cache — world-space irradiance probe grid for Lumen GI.
//!
//! Maintains a 3D grid of probes, each storing L2 spherical harmonics (9 coefficients
//! per RGB channel). A rolling subset of probes is updated each frame via a GPU compute
//! shader, providing temporally stable multi-bounce indirect lighting.
//!
//! The surface cache is the primary source of *far-field* diffuse GI. Near-field detail
//! comes from screen-space techniques (SSGI, GTAO) that are composited in the final
//! gather pass.

use bytemuck::{Pod, Zeroable};
use glam::{UVec3, Vec3};
use wgpu::util::DeviceExt;

// ---------------------------------------------------------------------------
// GPU types (must match WGSL struct layout)
// ---------------------------------------------------------------------------

/// GPU-side uniform parameters for the surface cache update compute pass.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct SurfaceCacheParams {
    pub grid_origin: [f32; 3],
    pub probe_spacing: f32,
    pub grid_dims: [u32; 3],
    pub num_probes: u32,
    pub update_offset: u32,
    pub update_count: u32,
    pub frame_index: u32,
    pub hysteresis: f32,
    pub sky_intensity: f32,
    pub _pad0: f32,
    pub _pad1: f32,
    pub _pad2: f32,
}

/// A single probe's L2 spherical harmonics: 9 coefficients × RGB + weight.
/// Each coefficient is stored as vec4(r, g, b, weight).
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct ProbeSH {
    pub c: [[f32; 4]; 9],
}

impl ProbeSH {
    pub const ZERO: Self = Self { c: [[0.0; 4]; 9] };
}

/// Directional light data uploaded to the surface cache update shader.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct DirectionalLightGpu {
    pub direction: [f32; 3],
    pub intensity: f32,
    pub color: [f32; 3],
    pub _pad: f32,
}

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Surface cache configuration.
#[derive(Debug, Clone)]
pub struct SurfaceCacheConfig {
    /// Number of probes per axis (x, y, z).
    pub grid_dims: UVec3,
    /// World-space distance between adjacent probes.
    pub probe_spacing: f32,
    /// World-space origin (min corner) of the probe grid.
    pub grid_origin: Vec3,
    /// Fraction of probes to update per frame (0.0–1.0).
    pub update_fraction: f32,
    /// Temporal hysteresis (higher = slower, more stable updates).
    pub hysteresis: f32,
    /// Sky light intensity multiplier.
    pub sky_intensity: f32,
}

impl Default for SurfaceCacheConfig {
    fn default() -> Self {
        Self {
            grid_dims: UVec3::new(16, 8, 16),
            probe_spacing: 4.0,
            grid_origin: Vec3::new(-32.0, -4.0, -32.0),
            update_fraction: 0.125, // update 12.5% of probes per frame (full cycle in 8 frames)
            hysteresis: 0.95,
            sky_intensity: 1.0,
        }
    }
}

impl SurfaceCacheConfig {
    pub fn total_probes(&self) -> u32 {
        self.grid_dims.x * self.grid_dims.y * self.grid_dims.z
    }
}

// ---------------------------------------------------------------------------
// Surface Cache Pass
// ---------------------------------------------------------------------------

/// Manages GPU resources for the surface cache probe grid.
pub struct SurfaceCachePass {
    config: SurfaceCacheConfig,
    /// Compute pipeline for updating probes.
    pipeline: wgpu::ComputePipeline,
    /// Bind group layout.
    bgl: wgpu::BindGroupLayout,
    /// Uniform params buffer.
    params_buf: wgpu::Buffer,
    /// Storage buffer holding all ProbeSH data.
    probe_buf: wgpu::Buffer,
    /// Storage buffer for directional lights.
    light_buf: wgpu::Buffer,
    /// Maximum lights allocated.
    max_lights: u32,
    /// Rolling update offset (wraps around total probes).
    update_cursor: u32,
    /// Frame counter.
    frame_index: u32,
    /// Cached linear sampler (reused across frames).
    sampler: wgpu::Sampler,
    /// Cached bind group (generation-tracked).
    cached_bg: crate::bind_group_cache::CachedBindGroup,
}

impl SurfaceCachePass {
    /// Create a new surface cache with the given configuration.
    pub fn new(device: &wgpu::Device, config: SurfaceCacheConfig) -> Self {
        let total = config.total_probes();
        let probe_data = vec![ProbeSH::ZERO; total as usize];
        let probe_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("surface_cache_probes"),
            contents: bytemuck::cast_slice(&probe_data),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        let params = SurfaceCacheParams {
            grid_origin: config.grid_origin.to_array(),
            probe_spacing: config.probe_spacing,
            grid_dims: config.grid_dims.to_array(),
            num_probes: total,
            update_offset: 0,
            update_count: 0,
            frame_index: 0,
            hysteresis: config.hysteresis,
            sky_intensity: config.sky_intensity,
            _pad0: 0.0,
            _pad1: 0.0,
            _pad2: 0.0,
        };
        let params_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("surface_cache_params"),
            contents: bytemuck::bytes_of(&params),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let max_lights: u32 = 64;
        let light_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("surface_cache_lights"),
            size: (max_lights as u64) * std::mem::size_of::<DirectionalLightGpu>() as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("surface_cache_bgl"),
            entries: &[
                // 0: params uniform
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
                // 1: probes (read_write storage)
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
                // 2: directional lights (read storage)
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // 3: depth texture
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                // 4: albedo texture
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                // 5: sampler
                wgpu::BindGroupLayoutEntry {
                    binding: 5,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("surface_cache_update_shader"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!("../shaders/lumen/surface_cache_update.wgsl").into(),
            ),
        });
        let pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("surface_cache_pl"),
            bind_group_layouts: &[&bgl],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("surface_cache_pipeline"),
            layout: Some(&pl),
            module: &shader,
            entry_point: Some("surface_cache_update"),
            compilation_options: Default::default(),
            cache: None,
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("surface_cache_sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        Self {
            config,
            pipeline,
            bgl,
            params_buf,
            probe_buf,
            light_buf,
            max_lights,
            update_cursor: 0,
            frame_index: 0,
            sampler,
            cached_bg: crate::bind_group_cache::CachedBindGroup::new(),
        }
    }

    /// Configuration accessor.
    pub fn config(&self) -> &SurfaceCacheConfig {
        &self.config
    }

    /// Get the probe storage buffer for read access in downstream passes.
    pub fn probe_buffer(&self) -> &wgpu::Buffer {
        &self.probe_buf
    }

    /// Get the bind group layout for creating compatible bind groups.
    pub fn bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.bgl
    }

    /// Total number of probes in the grid.
    pub fn total_probes(&self) -> u32 {
        self.config.total_probes()
    }

    /// Upload directional light data for this frame.
    pub fn upload_lights(&self, queue: &wgpu::Queue, lights: &[DirectionalLightGpu]) {
        let count = lights.len().min(self.max_lights as usize);
        if count > 0 {
            queue.write_buffer(&self.light_buf, 0, bytemuck::cast_slice(&lights[..count]));
        }
    }

    /// Update uniform parameters and advance the rolling update window.
    pub fn prepare_frame(&mut self, queue: &wgpu::Queue) {
        let total = self.config.total_probes();
        let update_count = ((total as f32 * self.config.update_fraction).ceil() as u32).min(total);

        let params = SurfaceCacheParams {
            grid_origin: self.config.grid_origin.to_array(),
            probe_spacing: self.config.probe_spacing,
            grid_dims: self.config.grid_dims.to_array(),
            num_probes: total,
            update_offset: self.update_cursor,
            update_count,
            frame_index: self.frame_index,
            hysteresis: self.config.hysteresis,
            sky_intensity: self.config.sky_intensity,
            _pad0: 0.0,
            _pad1: 0.0,
            _pad2: 0.0,
        };
        queue.write_buffer(&self.params_buf, 0, bytemuck::bytes_of(&params));

        // Advance cursor with wrap
        self.update_cursor = (self.update_cursor + update_count) % total;
        self.frame_index = self.frame_index.wrapping_add(1);
    }

    /// Dispatch the surface cache update compute pass.
    pub fn execute(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        depth_view: &wgpu::TextureView,
        albedo_view: &wgpu::TextureView,
        resource_gen: crate::bind_group_cache::Generation,
    ) {
        let total = self.config.total_probes();
        let update_count = ((total as f32 * self.config.update_fraction).ceil() as u32).min(total);

        if update_count == 0 {
            return;
        }

        let bg = self.cached_bg.get_or_rebuild(resource_gen, || {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("surface_cache_bg"),
                layout: &self.bgl,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: self.params_buf.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: self.probe_buf.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: self.light_buf.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::TextureView(depth_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: wgpu::BindingResource::TextureView(albedo_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 5,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                ],
            })
        });

        let workgroups = (update_count + 63) / 64;
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("surface_cache_update"),
            timestamp_writes: None,
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, bg, &[]);
        pass.dispatch_workgroups(workgroups, 1, 1);
    }

    /// Evaluate irradiance at a world-space position by trilinear interpolation
    /// of the 8 nearest probes (CPU fallback for debugging / validation).
    pub fn sample_irradiance_cpu(&self, probes: &[ProbeSH], pos: Vec3, normal: Vec3) -> Vec3 {
        let rel = (pos - self.config.grid_origin) / self.config.probe_spacing;
        let gx = rel.x.floor() as i32;
        let gy = rel.y.floor() as i32;
        let gz = rel.z.floor() as i32;
        let fx = rel.x.fract();
        let fy = rel.y.fract();
        let fz = rel.z.fract();

        let dims = self.config.grid_dims;
        let mut irradiance = Vec3::ZERO;

        for dz in 0..2i32 {
            for dy in 0..2i32 {
                for dx in 0..2i32 {
                    let ix = (gx + dx).clamp(0, dims.x as i32 - 1) as u32;
                    let iy = (gy + dy).clamp(0, dims.y as i32 - 1) as u32;
                    let iz = (gz + dz).clamp(0, dims.z as i32 - 1) as u32;
                    let idx = iz * dims.x * dims.y + iy * dims.x + ix;

                    let wx = if dx == 0 { 1.0 - fx } else { fx };
                    let wy = if dy == 0 { 1.0 - fy } else { fy };
                    let wz = if dz == 0 { 1.0 - fz } else { fz };
                    let w = wx * wy * wz;

                    if let Some(probe) = probes.get(idx as usize) {
                        let sh = evaluate_sh(probe, normal);
                        irradiance += sh * w;
                    }
                }
            }
        }

        irradiance.max(Vec3::ZERO)
    }
}

/// Evaluate SH irradiance for a given normal direction.
fn evaluate_sh(probe: &ProbeSH, dir: Vec3) -> Vec3 {
    let x = dir.x;
    let y = dir.y;
    let z = dir.z;

    // SH basis
    let b = [
        0.282095_f32,
        0.488603 * y,
        0.488603 * z,
        0.488603 * x,
        1.092548 * x * y,
        1.092548 * y * z,
        0.315392 * (3.0 * z * z - 1.0),
        1.092548 * x * z,
        0.546274 * (x * x - y * y),
    ];

    let mut result = Vec3::ZERO;
    for i in 0..9 {
        let c = probe.c[i];
        result += Vec3::new(c[0], c[1], c[2]) * b[i];
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn params_size() {
        // 3 floats + pad + 3 u32 + pad + 3 u32 + float + float + 3 pad = 64 bytes
        assert_eq!(std::mem::size_of::<SurfaceCacheParams>(), 64);
    }

    #[test]
    fn probe_sh_size() {
        // 9 × vec4 = 9 × 16 = 144 bytes
        assert_eq!(std::mem::size_of::<ProbeSH>(), 144);
    }

    #[test]
    fn dir_light_size() {
        assert_eq!(std::mem::size_of::<DirectionalLightGpu>(), 32);
    }

    #[test]
    fn default_config() {
        let c = SurfaceCacheConfig::default();
        assert_eq!(c.total_probes(), 16 * 8 * 16);
        assert_eq!(c.probe_spacing, 4.0);
        assert!((c.hysteresis - 0.95).abs() < 1e-6);
    }

    #[test]
    fn total_probes() {
        let c = SurfaceCacheConfig {
            grid_dims: UVec3::new(4, 4, 4),
            ..Default::default()
        };
        assert_eq!(c.total_probes(), 64);
    }

    #[test]
    fn sample_irradiance_zero_probes() {
        let config = SurfaceCacheConfig {
            grid_dims: UVec3::new(2, 2, 2),
            probe_spacing: 1.0,
            grid_origin: Vec3::ZERO,
            ..Default::default()
        };
        // Create a dummy pass just for CPU sampling (no GPU)
        // We test the function directly instead
        let probes = vec![ProbeSH::ZERO; 8];
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower,
            compatible_surface: None,
            force_fallback_adapter: false,
        }))
        .expect("adapter");
        let (device, _) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
                .expect("device");

        let pass = SurfaceCachePass::new(&device, config);
        let result = pass.sample_irradiance_cpu(&probes, Vec3::new(0.5, 0.5, 0.5), Vec3::Y);
        assert_eq!(result, Vec3::ZERO);
    }

    #[test]
    fn evaluate_sh_dc_only() {
        // A probe with only DC (c[0]) set should return uniform irradiance
        let mut probe = ProbeSH::ZERO;
        probe.c[0] = [1.0, 0.5, 0.25, 0.0];
        let result = evaluate_sh(&probe, Vec3::Y);
        // DC basis = 0.282095
        let expected = Vec3::new(1.0, 0.5, 0.25) * 0.282095;
        assert!((result - expected).length() < 1e-5);
    }

    #[test]
    fn evaluate_sh_directional() {
        // Probe with L1 Y component set — should give more light in +Y
        let mut probe = ProbeSH::ZERO;
        probe.c[0] = [1.0, 1.0, 1.0, 0.0]; // DC
        probe.c[1] = [1.0, 1.0, 1.0, 0.0]; // L1 Y
        let up = evaluate_sh(&probe, Vec3::Y);
        let down = evaluate_sh(&probe, Vec3::NEG_Y);
        // +Y should be brighter than -Y
        assert!(up.x > down.x);
    }

    #[test]
    fn surface_cache_pass_creation() {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower,
            compatible_surface: None,
            force_fallback_adapter: false,
        }))
        .expect("adapter");
        let (device, _) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
                .expect("device");

        let config = SurfaceCacheConfig {
            grid_dims: UVec3::new(4, 4, 4),
            ..Default::default()
        };
        let pass = SurfaceCachePass::new(&device, config);
        assert_eq!(pass.total_probes(), 64);
    }
}
