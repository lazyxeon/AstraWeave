//! GPU-instanced vegetation scatter and frustum cull pipeline.
//!
//! Two compute passes:
//! 1. **Scatter**: generates vegetation instances from a heightmap + biome data
//!    using a deterministic jittered-grid placement (GPU Poisson disk analog).
//! 2. **Cull**: frustum-culls the generated instances and writes survivors into
//!    a compacted draw-indirect buffer for instanced rendering.
//!
//! The CPU scatter path in `astraweave-terrain` remains available as a fallback.

use anyhow::Result;
use bytemuck::{Pod, Zeroable};
use glam::Vec3;

use crate::culling::FrustumPlanes;

// ── GPU structs (must match WGSL layout exactly) ────────────────────────────

/// Wind animation uniforms (32 bytes, matches WGSL `WindUniforms`).
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct WindUniforms {
    /// xy = wind direction (normalised), z = strength, w = time (seconds)
    pub wind_dir_strength_time: [f32; 4],
    /// x = trunk_sway_amplitude, y = trunk_sway_frequency,
    /// z = leaf_flutter_amplitude, w = leaf_flutter_frequency
    pub sway_params: [f32; 4],
}

impl Default for WindUniforms {
    fn default() -> Self {
        Self {
            wind_dir_strength_time: [std::f32::consts::FRAC_1_SQRT_2, std::f32::consts::FRAC_1_SQRT_2, 1.0, 0.0],
            sway_params: [0.02, 0.5, 0.05, 3.0],
        }
    }
}

/// Scatter parameters uniform (64 bytes, 16-byte aligned).
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct ScatterParams {
    pub chunk_origin_x: f32,
    pub chunk_origin_z: f32,
    pub chunk_size: f32,
    pub heightmap_res: u32,

    pub grid_dim: u32,
    pub min_distance: f32,
    pub max_slope: f32,
    pub seed: u32,

    pub density: f32,
    pub altitude_ceiling: f32,
    pub num_types: u32,
    pub max_instances: u32,

    pub _pad0: u32,
    pub _pad1: u32,
    pub _pad2: u32,
    pub _pad3: u32,
}

/// Per-vegetation-type info (16 bytes).
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct VegetationTypeGpu {
    pub scale_min: f32,
    pub scale_max: f32,
    pub slope_tolerance: f32,
    pub weight: f32,
}

/// GPU vegetation instance (32 bytes, matches WGSL `VegetationInstance`).
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct VegetationInstanceGpu {
    /// xyz = world position, w = scale
    pub pos_scale: [f32; 4],
    /// x = rotation (radians), y = type_index (as f32), z = normal.x, w = normal.y
    pub rot_type_normal: [f32; 4],
}

/// DrawIndexedIndirectCommand (20 bytes, matches wgpu layout).
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct DrawIndexedIndirectCommand {
    pub index_count: u32,
    pub instance_count: u32,
    pub first_index: u32,
    pub base_vertex: i32,
    pub first_instance: u32,
}

// ── Pipeline ────────────────────────────────────────────────────────────────

/// Maximum instances per chunk. Provides a hard cap for buffer sizing.
const MAX_INSTANCES_PER_CHUNK: u32 = 65_536;

/// Scatter compute shader source (loaded at compile time).
const SCATTER_SHADER_SRC: &str = include_str!("../shaders/vegetation_scatter.wgsl");

/// Manages the GPU vegetation scatter and cull compute pipelines.
pub struct VegetationGpuPipeline {
    // Scatter pass
    scatter_pipeline: wgpu::ComputePipeline,
    scatter_bind_group_layout: wgpu::BindGroupLayout,

    // Cull pass
    cull_pipeline: wgpu::ComputePipeline,
    cull_bind_group_layout: wgpu::BindGroupLayout,

    // Shared buffers (resized on demand)
    instance_buffer: wgpu::Buffer,
    instance_count_buffer: wgpu::Buffer,
    visible_instance_buffer: wgpu::Buffer,
    draw_cmd_buffer: wgpu::Buffer,
    params_buffer: wgpu::Buffer,
    frustum_buffer: wgpu::Buffer,
    cull_count_buffer: wgpu::Buffer,

    // Vegetation type info buffer
    veg_types_buffer: wgpu::Buffer,

    max_instances: u32,

    /// Sampler for heightmap texture
    heightmap_sampler: wgpu::Sampler,
}

impl VegetationGpuPipeline {
    /// Create the vegetation GPU pipeline.
    pub fn new(device: &wgpu::Device, max_instances: Option<u32>) -> Result<Self> {
        let max_instances = max_instances.unwrap_or(MAX_INSTANCES_PER_CHUNK);

        // ── Shader module ───────────────────────────────────────────────────
        let shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("vegetation_scatter.wgsl"),
            source: wgpu::ShaderSource::Wgsl(SCATTER_SHADER_SRC.into()),
        });

        // ── Scatter bind group layout ────────────────────────────────────────
        let scatter_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("VegScatter BGL"),
                entries: &[
                    // 0: ScatterParams uniform
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
                    // 1: Heightmap texture
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    // 2: Heightmap sampler
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                    // 3: VegetationType info (storage, read)
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // 4: Instance output buffer (storage, rw)
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
                    // 5: Atomic instance count (storage, rw)
                    wgpu::BindGroupLayoutEntry {
                        binding: 5,
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

        // ── Cull bind group layout ───────────────────────────────────────────
        let cull_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("VegCull BGL"),
                entries: &[
                    // 0: FrustumPlanes uniform
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
                    // 1: All instances (storage, read)
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
                    // 2: Instance count (uniform)
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // 3: Visible instances output (storage, rw)
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
                    // 4: DrawIndexedIndirectCommand (storage, rw)
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

        // ── Pipeline layouts ─────────────────────────────────────────────────
        let scatter_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("VegScatter PipelineLayout"),
                bind_group_layouts: &[&scatter_bind_group_layout],
                push_constant_ranges: &[],
            });

        let cull_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("VegCull PipelineLayout"),
                bind_group_layouts: &[&cull_bind_group_layout],
                push_constant_ranges: &[],
            });

        // ── Compute pipelines ────────────────────────────────────────────────
        let scatter_pipeline =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("VegScatter Pipeline"),
                layout: Some(&scatter_pipeline_layout),
                module: &shader_module,
                entry_point: Some("scatter_vegetation"),
                compilation_options: Default::default(),
                cache: None,
            });

        let cull_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("VegCull Pipeline"),
            layout: Some(&cull_pipeline_layout),
            module: &shader_module,
            entry_point: Some("cull_vegetation"),
            compilation_options: Default::default(),
            cache: None,
        });

        // ── Buffers ──────────────────────────────────────────────────────────
        let instance_byte_size =
            (max_instances as u64) * std::mem::size_of::<VegetationInstanceGpu>() as u64;

        let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("VegScatter Instances"),
            size: instance_byte_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let instance_count_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("VegScatter InstanceCount"),
            size: 4,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::UNIFORM,
            mapped_at_creation: false,
        });

        let visible_instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("VegCull VisibleInstances"),
            size: instance_byte_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::VERTEX,
            mapped_at_creation: false,
        });

        let draw_cmd_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("VegCull DrawCmd"),
            size: std::mem::size_of::<DrawIndexedIndirectCommand>() as u64,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::INDIRECT
                | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let params_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("VegScatter Params"),
            size: std::mem::size_of::<ScatterParams>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let frustum_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("VegCull Frustum"),
            size: std::mem::size_of::<FrustumPlanes>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let cull_count_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("VegCull Count"),
            size: 4,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Vegetation type info (up to 16 types)
        let veg_types_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("VegScatter Types"),
            size: 16 * std::mem::size_of::<VegetationTypeGpu>() as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let heightmap_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("VegScatter HeightmapSampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        Ok(Self {
            scatter_pipeline,
            scatter_bind_group_layout,
            cull_pipeline,
            cull_bind_group_layout,
            instance_buffer,
            instance_count_buffer,
            visible_instance_buffer,
            draw_cmd_buffer,
            params_buffer,
            frustum_buffer,
            cull_count_buffer,
            veg_types_buffer,
            max_instances,
            heightmap_sampler,
        })
    }

    /// Run the scatter compute pass for a single chunk.
    ///
    /// `heightmap_view`: the chunk's R32Float heightmap texture view.
    /// `veg_types`: per-species GPU data (max 16).
    pub fn dispatch_scatter(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        params: &ScatterParams,
        heightmap_view: &wgpu::TextureView,
        veg_types: &[VegetationTypeGpu],
    ) {
        // Upload params
        queue.write_buffer(&self.params_buffer, 0, bytemuck::bytes_of(params));

        // Upload vegetation types (pad to at least 1 entry)
        let type_count = veg_types.len().clamp(1, 16);
        let mut padded = [VegetationTypeGpu::zeroed(); 16];
        for (i, t) in veg_types.iter().take(type_count).enumerate() {
            padded[i] = *t;
        }
        queue.write_buffer(&self.veg_types_buffer, 0, bytemuck::cast_slice(&padded));

        // Clear instance count to 0
        queue.write_buffer(&self.instance_count_buffer, 0, &0u32.to_le_bytes());

        // Build scatter bind group
        let scatter_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("VegScatter BG"),
            layout: &self.scatter_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.params_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(heightmap_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&self.heightmap_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: self.veg_types_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: self.instance_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: self.instance_count_buffer.as_entire_binding(),
                },
            ],
        });

        // Dispatch: one thread per grid cell
        let total_cells = params.grid_dim * params.grid_dim;
        let workgroups = total_cells.div_ceil(64);

        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("VegScatter Pass"),
            timestamp_writes: None,
        });
        pass.set_pipeline(&self.scatter_pipeline);
        pass.set_bind_group(0, &scatter_bg, &[]);
        pass.dispatch_workgroups(workgroups, 1, 1);
    }

    /// Run the frustum cull compute pass on previously scattered instances.
    ///
    /// Call after `dispatch_scatter`. The visible instances are written to
    /// `visible_instance_buffer` and `draw_cmd_buffer.instance_count` is
    /// set atomically.
    ///
    /// `index_count`: number of indices in the mesh used for draw-indirect.
    pub fn dispatch_cull(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        frustum: &FrustumPlanes,
        scattered_count: u32,
        index_count: u32,
    ) {
        // Upload frustum planes
        queue.write_buffer(&self.frustum_buffer, 0, bytemuck::bytes_of(frustum));

        // Upload instance count for cull shader
        queue.write_buffer(&self.cull_count_buffer, 0, &scattered_count.to_le_bytes());

        // Initialize draw command: instance_count = 0, rest filled in
        let initial_cmd = DrawIndexedIndirectCommand {
            index_count,
            instance_count: 0,
            first_index: 0,
            base_vertex: 0,
            first_instance: 0,
        };
        queue.write_buffer(&self.draw_cmd_buffer, 0, bytemuck::bytes_of(&initial_cmd));

        // Build cull bind group
        let cull_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("VegCull BG"),
            layout: &self.cull_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.frustum_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.instance_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.cull_count_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: self.visible_instance_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: self.draw_cmd_buffer.as_entire_binding(),
                },
            ],
        });

        let workgroups = scattered_count.div_ceil(64);

        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("VegCull Pass"),
            timestamp_writes: None,
        });
        pass.set_pipeline(&self.cull_pipeline);
        pass.set_bind_group(0, &cull_bg, &[]);
        pass.dispatch_workgroups(workgroups, 1, 1);
    }

    /// Get the draw-indirect buffer for use with `draw_indexed_indirect()`.
    pub fn draw_cmd_buffer(&self) -> &wgpu::Buffer {
        &self.draw_cmd_buffer
    }

    /// Get the visible instance buffer to bind as a vertex buffer for instanced rendering.
    pub fn visible_instance_buffer(&self) -> &wgpu::Buffer {
        &self.visible_instance_buffer
    }

    /// Get the instance buffer (all scattered, pre-cull) for readback or debugging.
    pub fn instance_buffer(&self) -> &wgpu::Buffer {
        &self.instance_buffer
    }

    /// Get maximum instances capacity.
    pub fn max_instances(&self) -> u32 {
        self.max_instances
    }

    /// Returns the vertex buffer layout for `VegetationInstanceGpu` in instance mode.
    ///
    /// Shader locations 9-10 (after model matrix at 5-8):
    ///   location 9: pos_scale (vec4<f32>)
    ///   location 10: rot_type_normal (vec4<f32>)
    pub fn instance_vertex_layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<VegetationInstanceGpu>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 9,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: 16,
                    shader_location: 10,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}

// ── Helper: build ScatterParams from terrain data ───────────────────────────

/// Convenience builder for `ScatterParams`.
pub struct ScatterParamsBuilder {
    pub chunk_origin: [f32; 2],
    pub chunk_size: f32,
    pub heightmap_res: u32,
    pub grid_dim: u32,
    pub min_distance: f32,
    pub max_slope: f32,
    pub seed: u32,
    pub density: f32,
    pub altitude_ceiling: f32,
    pub num_types: u32,
    pub max_instances: u32,
}

impl Default for ScatterParamsBuilder {
    fn default() -> Self {
        Self {
            chunk_origin: [0.0, 0.0],
            chunk_size: 256.0,
            heightmap_res: 129,
            grid_dim: 64,
            min_distance: 2.0,
            max_slope: 35.0,
            seed: 42,
            density: 1.0,
            altitude_ceiling: f32::MAX,
            num_types: 1,
            max_instances: MAX_INSTANCES_PER_CHUNK,
        }
    }
}

impl ScatterParamsBuilder {
    pub fn build(&self) -> ScatterParams {
        ScatterParams {
            chunk_origin_x: self.chunk_origin[0],
            chunk_origin_z: self.chunk_origin[1],
            chunk_size: self.chunk_size,
            heightmap_res: self.heightmap_res,
            grid_dim: self.grid_dim,
            min_distance: self.min_distance,
            max_slope: self.max_slope,
            seed: self.seed,
            density: self.density,
            altitude_ceiling: self.altitude_ceiling,
            num_types: self.num_types,
            max_instances: self.max_instances,
            _pad0: 0,
            _pad1: 0,
            _pad2: 0,
            _pad3: 0,
        }
    }
}

// ── CPU fallback ────────────────────────────────────────────────────────────

/// CPU-side vegetation scatter for debugging / fallback.
///
/// Mirrors the GPU algorithm for deterministic parity when GPU compute is
/// unavailable. Uses the same PCG hash for reproducibility.
pub fn cpu_scatter_vegetation(
    params: &ScatterParams,
    heights: &[f32],
    veg_types: &[VegetationTypeGpu],
) -> Vec<VegetationInstanceGpu> {
    let mut instances = Vec::new();
    let total_cells = params.grid_dim * params.grid_dim;

    for cell_index in 0..total_cells {
        let cell_x = cell_index % params.grid_dim;
        let cell_z = cell_index / params.grid_dim;

        let cell_seed = pcg_hash(cell_index ^ params.seed);

        // Density rejection
        let density_roll = hash_to_float(pcg_hash(cell_seed.wrapping_add(1)));
        if density_roll > params.density {
            continue;
        }

        let cell_size = params.chunk_size / params.grid_dim as f32;
        let jitter_x = hash_to_float(pcg_hash(cell_seed.wrapping_add(2)));
        let jitter_z = hash_to_float(pcg_hash(cell_seed.wrapping_add(3)));

        let local_x = (cell_x as f32 + jitter_x) * cell_size;
        let local_z = (cell_z as f32 + jitter_z) * cell_size;

        // Sample height from flat array
        let u = local_x / params.chunk_size;
        let v = local_z / params.chunk_size;
        if !(0.0..=1.0).contains(&u) || !(0.0..=1.0).contains(&v) {
            continue;
        }

        let res = params.heightmap_res as f32;
        let px = (u * (res - 1.0)).min(res - 2.0);
        let pz = (v * (res - 1.0)).min(res - 2.0);
        let ix = px as usize;
        let iz = pz as usize;
        let fx = px - ix as f32;
        let fz = pz - iz as f32;
        let stride = params.heightmap_res as usize;

        if iz * stride + ix + stride + 1 >= heights.len() {
            continue;
        }

        let h00 = heights[iz * stride + ix];
        let h10 = heights[iz * stride + ix + 1];
        let h01 = heights[(iz + 1) * stride + ix];
        let h11 = heights[(iz + 1) * stride + ix + 1];
        let height = h00 * (1.0 - fx) * (1.0 - fz)
            + h10 * fx * (1.0 - fz)
            + h01 * (1.0 - fx) * fz
            + h11 * fx * fz;

        if height > params.altitude_ceiling {
            continue;
        }

        // Slope estimation via central differences
        let texel = 1.0 / res;
        let world_step = params.chunk_size * texel;
        let sample_h = |su: f32, sv: f32| -> f32 {
            let su = su.clamp(0.0, 1.0);
            let sv = sv.clamp(0.0, 1.0);
            let spx = (su * (res - 1.0)).min(res - 2.0);
            let spz = (sv * (res - 1.0)).min(res - 2.0);
            let six = spx as usize;
            let siz = spz as usize;
            let sfx = spx - six as f32;
            let sfz = spz - siz as f32;
            if siz * stride + six + stride + 1 >= heights.len() {
                return 0.0;
            }
            heights[siz * stride + six] * (1.0 - sfx) * (1.0 - sfz)
                + heights[siz * stride + six + 1] * sfx * (1.0 - sfz)
                + heights[(siz + 1) * stride + six] * (1.0 - sfx) * sfz
                + heights[(siz + 1) * stride + six + 1] * sfx * sfz
        };

        let h_l = sample_h(u - texel, v);
        let h_r = sample_h(u + texel, v);
        let h_d = sample_h(u, v - texel);
        let h_u = sample_h(u, v + texel);

        let dx = (h_r - h_l) / (2.0 * world_step);
        let dz = (h_u - h_d) / (2.0 * world_step);

        let normal = Vec3::new(-dx, 1.0, -dz).normalize();
        let slope_cos = normal.y;
        let max_slope_cos = params.max_slope.to_radians().cos();
        if slope_cos < max_slope_cos {
            continue;
        }

        // Select type
        let type_roll = hash_to_float(pcg_hash(cell_seed.wrapping_add(4)));
        let mut accum_weight = 0.0f32;
        let mut selected_type = 0u32;
        let n_types = (params.num_types as usize).min(veg_types.len()).min(16);
        for (t, veg_type) in veg_types.iter().enumerate().take(n_types) {
            accum_weight += veg_type.weight;
            if type_roll < accum_weight {
                selected_type = t as u32;
                break;
            }
            selected_type = t as u32;
        }

        // Per-type slope tolerance
        if n_types > 0 {
            let tol = veg_types[selected_type as usize].slope_tolerance;
            if tol > 0.0 && slope_cos < tol.to_radians().cos() {
                continue;
            }
        }

        // Scale + rotation
        let scale_min = if n_types > 0 {
            veg_types[selected_type as usize].scale_min
        } else {
            0.8
        };
        let scale_max = if n_types > 0 {
            veg_types[selected_type as usize].scale_max
        } else {
            1.2
        };
        let scale_t = hash_to_float(pcg_hash(cell_seed.wrapping_add(5)));
        let instance_scale = scale_min + (scale_max - scale_min) * scale_t;
        let rotation = hash_to_float(pcg_hash(cell_seed.wrapping_add(6))) * std::f32::consts::TAU;

        let world_x = params.chunk_origin_x + local_x;
        let world_z = params.chunk_origin_z + local_z;

        instances.push(VegetationInstanceGpu {
            pos_scale: [world_x, height, world_z, instance_scale],
            rot_type_normal: [rotation, selected_type as f32, normal.x, normal.y],
        });

        if instances.len() >= params.max_instances as usize {
            break;
        }
    }

    instances
}

/// CPU frustum cull of vegetation instances.
pub fn cpu_cull_vegetation(
    instances: &[VegetationInstanceGpu],
    frustum: &FrustumPlanes,
) -> Vec<VegetationInstanceGpu> {
    let frustum_glam = crate::culling::FrustumPlanes {
        planes: frustum.planes,
    };
    instances
        .iter()
        .filter(|inst| {
            let pos = Vec3::new(inst.pos_scale[0], inst.pos_scale[1], inst.pos_scale[2]);
            let radius = inst.pos_scale[3] * 1.732;
            let extent = Vec3::splat(radius);
            frustum_glam.test_aabb(pos, extent)
        })
        .copied()
        .collect()
}

// ── PCG hash (matches WGSL) ─────────────────────────────────────────────────

fn pcg_hash(input: u32) -> u32 {
    let state = input.wrapping_mul(747796405).wrapping_add(2891336453);
    let word = ((state >> ((state >> 28).wrapping_add(4))) ^ state).wrapping_mul(277803737);
    (word >> 22) ^ word
}

fn hash_to_float(h: u32) -> f32 {
    h as f32 / 4294967295.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scatter_params_size() {
        // Must be 64 bytes (4 × vec4) for std140 alignment
        assert_eq!(std::mem::size_of::<ScatterParams>(), 64);
    }

    #[test]
    fn test_vegetation_instance_gpu_size() {
        assert_eq!(std::mem::size_of::<VegetationInstanceGpu>(), 32);
    }

    #[test]
    fn test_vegetation_type_gpu_size() {
        assert_eq!(std::mem::size_of::<VegetationTypeGpu>(), 16);
    }

    #[test]
    fn test_draw_indexed_indirect_size() {
        assert_eq!(std::mem::size_of::<DrawIndexedIndirectCommand>(), 20);
    }

    #[test]
    fn test_pcg_hash_deterministic() {
        assert_eq!(pcg_hash(0), pcg_hash(0));
        assert_ne!(pcg_hash(0), pcg_hash(1));
    }

    #[test]
    fn test_hash_to_float_range() {
        for i in 0..1000 {
            let f = hash_to_float(pcg_hash(i));
            assert!(f >= 0.0 && f <= 1.0, "hash_to_float out of range: {f}");
        }
    }

    #[test]
    fn test_cpu_scatter_empty_heightmap() {
        let params = ScatterParamsBuilder::default().build();
        let veg_types = vec![VegetationTypeGpu {
            scale_min: 0.8,
            scale_max: 1.2,
            slope_tolerance: 0.0,
            weight: 1.0,
        }];
        // 129×129 flat heightmap at y=50
        let heights = vec![50.0f32; 129 * 129];
        let instances = cpu_scatter_vegetation(&params, &heights, &veg_types);
        // Should produce instances on flat terrain
        assert!(!instances.is_empty(), "flat terrain should produce instances");
        // All instances should be at height 50
        for inst in &instances {
            assert!(
                (inst.pos_scale[1] - 50.0).abs() < 1.0,
                "instance height should be ~50.0, got {}",
                inst.pos_scale[1]
            );
        }
    }

    #[test]
    fn test_cpu_scatter_respects_density() {
        let full = ScatterParamsBuilder {
            density: 1.0,
            ..Default::default()
        }
        .build();

        let half = ScatterParamsBuilder {
            density: 0.5,
            ..Default::default()
        }
        .build();

        let veg_types = vec![VegetationTypeGpu {
            scale_min: 0.8,
            scale_max: 1.2,
            slope_tolerance: 0.0,
            weight: 1.0,
        }];
        let heights = vec![50.0f32; 129 * 129];

        let full_count = cpu_scatter_vegetation(&full, &heights, &veg_types).len();
        let half_count = cpu_scatter_vegetation(&half, &heights, &veg_types).len();

        // Half density should produce roughly half the instances (within 30% tolerance)
        let ratio = half_count as f32 / full_count as f32;
        assert!(
            ratio > 0.3 && ratio < 0.7,
            "half density ratio should be ~0.5, got {ratio}"
        );
    }

    #[test]
    fn test_cpu_scatter_altitude_ceiling() {
        let params = ScatterParamsBuilder {
            altitude_ceiling: 40.0,
            ..Default::default()
        }
        .build();
        let veg_types = vec![VegetationTypeGpu {
            scale_min: 0.8,
            scale_max: 1.2,
            slope_tolerance: 0.0,
            weight: 1.0,
        }];
        // All above ceiling
        let heights = vec![50.0f32; 129 * 129];
        let instances = cpu_scatter_vegetation(&params, &heights, &veg_types);
        assert!(
            instances.is_empty(),
            "above-ceiling terrain should produce zero instances"
        );
    }

    #[test]
    fn test_cpu_cull_behind_camera() {
        let instances = vec![VegetationInstanceGpu {
            pos_scale: [0.0, 0.0, 100.0, 1.0], // positive Z = behind in RH
            rot_type_normal: [0.0, 0.0, 0.0, 1.0],
        }];

        // perspective_rh looks down -Z: objects at +Z are behind
        let vp = glam::Mat4::perspective_rh(1.0, 1.0, 0.1, 1000.0);
        let frustum = FrustumPlanes::from_view_proj(&vp);
        let visible = cpu_cull_vegetation(&instances, &frustum);
        assert!(
            visible.is_empty(),
            "instance behind camera should be culled"
        );
    }

    #[test]
    fn test_cpu_cull_in_front() {
        let instances = vec![VegetationInstanceGpu {
            pos_scale: [0.0, 0.0, -10.0, 1.0], // negative Z = in front in RH
            rot_type_normal: [0.0, 0.0, 0.0, 1.0],
        }];

        let vp = glam::Mat4::perspective_rh(1.0, 1.0, 0.1, 1000.0);
        let frustum = FrustumPlanes::from_view_proj(&vp);
        let visible = cpu_cull_vegetation(&instances, &frustum);
        assert_eq!(
            visible.len(),
            1,
            "instance in front of camera should be visible"
        );
    }

    #[test]
    fn test_scatter_params_builder_defaults() {
        let p = ScatterParamsBuilder::default().build();
        assert_eq!(p.chunk_size, 256.0);
        assert_eq!(p.grid_dim, 64);
        assert_eq!(p.min_distance, 2.0);
        assert_eq!(p.max_slope, 35.0);
        assert_eq!(p.density, 1.0);
    }
}
