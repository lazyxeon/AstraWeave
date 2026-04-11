//! Geometry Clipmap / CDLOD terrain rendering system.
//!
//! **DEPRECATED**: This module is superseded by the terrain crate's chunk-based
//! `LodManager` + `MorphingLodManager`, which integrates streaming, SVO-based
//! voxel LOD, erosion, and cave density. Use `astraweave_terrain::LodManager`
//! with `MorphingLodManager` for new terrain rendering pipelines.
//!
//! Implements concentric rings of terrain mesh centered on the camera, where
//! each successive ring doubles the cell size. Inner rings are fine-grained;
//! outer rings cover vast distances with fewer vertices.
//!
#![allow(deprecated)]
//! # Architecture
//!
//! ```text
//! Ring 0 (finest)   ─ cell_size = base_scale
//! Ring 1            ─ cell_size = base_scale × 2
//! Ring 2            ─ cell_size = base_scale × 4
//! ...
//! Ring N (coarsest) ─ cell_size = base_scale × 2^N
//! ```
//!
//! Each ring is a square grid with an inner hole cut out (except ring 0).
//! Vertices morph toward the parent LOD grid at distance boundaries to
//! eliminate popping.
//!
//! Reference: Strugar (2009) "Continuous Distance-Dependent LOD",
//! Losasso & Hoppe (2004) "Geometry Clipmaps".

use bytemuck::{Pod, Zeroable};

/// WGSL shader source for clipmap terrain rendering.
pub const CLIPMAP_TERRAIN_WGSL: &str = include_str!("../shaders/clipmap_terrain.wgsl");

/// Maximum number of clipmap rings (levels).
pub const MAX_CLIPMAP_LEVELS: usize = 10;

/// Configuration for the geometry clipmap system.
#[derive(Debug, Clone)]
pub struct ClipmapConfig {
    /// Number of concentric rings (LOD levels). Each ring doubles cell size.
    pub num_levels: u32,
    /// Grid size per ring (vertices along one edge). Must be odd ≥ 3.
    /// Typical: 63 or 127. Higher = more vertices per ring but smoother.
    pub grid_size: u32,
    /// World-space size of the finest grid cell (ring 0).
    pub base_cell_size: f32,
    /// Fraction of ring extent at which morphing begins (0.6 = start morph
    /// at 60% of ring's outer boundary).
    pub morph_start_fraction: f32,
    /// Total heightmap width in texels (for UV mapping).
    pub heightmap_width: f32,
    /// Total heightmap height in texels.
    pub heightmap_height: f32,
}

impl Default for ClipmapConfig {
    fn default() -> Self {
        Self {
            num_levels: 6,
            grid_size: 63,
            base_cell_size: 1.0,
            morph_start_fraction: 0.6,
            heightmap_width: 4096.0,
            heightmap_height: 4096.0,
        }
    }
}

impl ClipmapConfig {
    /// Cell size at a given ring level.
    pub fn cell_size(&self, level: u32) -> f32 {
        self.base_cell_size * (1u32 << level) as f32
    }

    /// World-space extent of a ring (half-width of the grid).
    pub fn ring_extent(&self, level: u32) -> f32 {
        self.cell_size(level) * (self.grid_size as f32) * 0.5
    }

    /// Distance at which morphing starts for a given level.
    pub fn morph_start_distance(&self, level: u32) -> f32 {
        self.ring_extent(level) * self.morph_start_fraction
    }

    /// Inverse of morph range for shader use: 1 / (morph_end - morph_start).
    pub fn morph_range_inv(&self, level: u32) -> f32 {
        let extent = self.ring_extent(level);
        let start = extent * self.morph_start_fraction;
        let range = extent - start;
        if range > 0.0 {
            1.0 / range
        } else {
            1.0
        }
    }

    /// Validate configuration. Returns Err with message if invalid.
    pub fn validate(&self) -> Result<(), String> {
        if self.num_levels == 0 || self.num_levels as usize > MAX_CLIPMAP_LEVELS {
            return Err(format!(
                "num_levels must be 1..={MAX_CLIPMAP_LEVELS}, got {}",
                self.num_levels
            ));
        }
        if self.grid_size < 3 || self.grid_size % 2 == 0 {
            return Err(format!(
                "grid_size must be odd and >= 3, got {}",
                self.grid_size
            ));
        }
        if self.base_cell_size <= 0.0 {
            return Err(format!(
                "base_cell_size must be > 0, got {}",
                self.base_cell_size
            ));
        }
        if !(0.0..=1.0).contains(&self.morph_start_fraction) {
            return Err(format!(
                "morph_start_fraction must be in [0,1], got {}",
                self.morph_start_fraction
            ));
        }
        Ok(())
    }
}

/// Per-vertex data for clipmap grid (CPU-side, uploaded to vertex buffer).
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct ClipmapVertex {
    /// Integer grid coordinates within the ring (centered at 0,0).
    pub grid_pos: [f32; 2],
    /// (ring_level, ring_scale, morph_start_dist, morph_range_inv)
    pub ring_info: [f32; 4],
}

const _: () = assert!(std::mem::size_of::<ClipmapVertex>() == 24);

/// Uniform data for the clipmap shader.
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct ClipmapUniforms {
    /// View-projection matrix (column-major).
    pub view_proj: [[f32; 4]; 4],
    /// Camera world position.
    /// When using `camera-relative` rendering, set to `Vec3::ZERO`; the
    /// actual world position goes in `heightmap_origin` instead.
    pub camera_pos: [f32; 3],
    /// Finest grid cell size.
    pub clipmap_scale: f32,
    /// Morph constants: (morph_start, morph_range_inv, 0, 0)
    pub morph_constants: [f32; 4],
    /// Heightmap size.
    pub heightmap_size: [f32; 2],
    /// 1/heightmap size.
    pub inv_heightmap_size: [f32; 2],
    /// True world-space XZ position of the camera, used to offset heightmap UV
    /// lookups. In standard rendering this should be `[0.0, 0.0]` (world_xz
    /// already contains the absolute position). In `camera-relative` mode,
    /// set to the camera's DVec3 XZ downcast to f32 so height sampling uses
    /// the correct world-space location while vertices stay near the origin.
    pub heightmap_origin: [f32; 2],
    pub _pad: [f32; 2],
}

const _: () = assert!(std::mem::size_of::<ClipmapUniforms>() == 128);

/// Pre-built mesh data for a single clipmap ring.
#[derive(Debug)]
pub struct RingMesh {
    /// Vertices for this ring.
    pub vertices: Vec<ClipmapVertex>,
    /// Triangle indices.
    pub indices: Vec<u32>,
    /// Ring level (0 = finest).
    pub level: u32,
}

/// Generates GPU-ready mesh data for all clipmap rings.
///
/// Ring 0 is a solid grid. Rings 1..N are hollow (inner portion cut out
/// since it's covered by the finer ring).
pub fn generate_clipmap_rings(config: &ClipmapConfig) -> Vec<RingMesh> {
    let n = config.grid_size as i32;
    let half = n / 2;
    let mut rings = Vec::with_capacity(config.num_levels as usize);

    for level in 0..config.num_levels {
        let scale = (1u32 << level) as f32;
        let morph_start = config.morph_start_distance(level);
        let morph_range_inv = config.morph_range_inv(level);

        let inner_half = if level == 0 { 0 } else { half / 2 };

        let mut vertices = Vec::new();
        let mut vertex_map: std::collections::HashMap<(i32, i32), u32> =
            std::collections::HashMap::new();

        // Generate vertices for the grid, skipping the inner hole
        for z in -half..=half {
            for x in -half..=half {
                // Skip inner region (covered by finer ring)
                if level > 0 && x.abs() <= inner_half && z.abs() <= inner_half {
                    continue;
                }
                let idx = vertices.len() as u32;
                vertex_map.insert((x, z), idx);
                vertices.push(ClipmapVertex {
                    grid_pos: [x as f32, z as f32],
                    ring_info: [level as f32, scale, morph_start, morph_range_inv],
                });
            }
        }

        // Generate triangles (two per quad)
        let mut indices = Vec::new();
        for z in -half..half {
            for x in -half..half {
                // Skip quads whose vertices would be inside the inner hole
                let corners = [(x, z), (x + 1, z), (x + 1, z + 1), (x, z + 1)];
                let all_present = corners.iter().all(|c| vertex_map.contains_key(c));
                if !all_present {
                    continue;
                }

                let i00 = vertex_map[&(x, z)];
                let i10 = vertex_map[&(x + 1, z)];
                let i11 = vertex_map[&(x + 1, z + 1)];
                let i01 = vertex_map[&(x, z + 1)];

                // Triangle 1
                indices.push(i00);
                indices.push(i10);
                indices.push(i11);
                // Triangle 2
                indices.push(i00);
                indices.push(i11);
                indices.push(i01);
            }
        }

        rings.push(RingMesh {
            vertices,
            indices,
            level,
        });
    }

    rings
}

/// GPU resources for the clipmap terrain system.
pub struct ClipmapTerrain {
    /// Vertex buffers per ring level.
    ring_vertex_buffers: Vec<wgpu::Buffer>,
    /// Index buffers per ring level.
    ring_index_buffers: Vec<wgpu::Buffer>,
    /// Index counts per ring level.
    ring_index_counts: Vec<u32>,
    /// Uniform buffer.
    uniform_buffer: wgpu::Buffer,
    /// Bind group layout.
    bind_group_layout: wgpu::BindGroupLayout,
    /// Bind group.
    bind_group: wgpu::BindGroup,
    /// Render pipeline.
    pipeline: wgpu::RenderPipeline,
    /// Configuration.
    config: ClipmapConfig,
}

impl ClipmapTerrain {
    /// Create the clipmap terrain system.
    ///
    /// `heightmap_view` is the texture view for the heightmap (R32Float or R16Float).
    /// `surface_format` is the render target format.
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        config: &ClipmapConfig,
        heightmap_view: &wgpu::TextureView,
        surface_format: wgpu::TextureFormat,
        depth_format: wgpu::TextureFormat,
    ) -> Self {
        // Generate ring meshes
        let rings = generate_clipmap_rings(config);

        let mut ring_vertex_buffers = Vec::with_capacity(rings.len());
        let mut ring_index_buffers = Vec::with_capacity(rings.len());
        let mut ring_index_counts = Vec::with_capacity(rings.len());

        for ring in &rings {
            let vb = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(&format!("clipmap_ring{}_vb", ring.level)),
                size: (ring.vertices.len() * std::mem::size_of::<ClipmapVertex>()) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            queue.write_buffer(&vb, 0, bytemuck::cast_slice(&ring.vertices));

            let ib = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(&format!("clipmap_ring{}_ib", ring.level)),
                size: (ring.indices.len() * std::mem::size_of::<u32>()) as u64,
                usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            queue.write_buffer(&ib, 0, bytemuck::cast_slice(&ring.indices));

            ring_vertex_buffers.push(vb);
            ring_index_buffers.push(ib);
            ring_index_counts.push(ring.indices.len() as u32);
        }

        // Uniform buffer
        let uniforms = ClipmapUniforms {
            view_proj: [[0.0; 4]; 4],
            camera_pos: [0.0; 3],
            clipmap_scale: config.base_cell_size,
            morph_constants: [0.0; 4],
            heightmap_size: [config.heightmap_width, config.heightmap_height],
            inv_heightmap_size: [1.0 / config.heightmap_width, 1.0 / config.heightmap_height],
            heightmap_origin: [0.0; 2],
            _pad: [0.0; 2],
        };
        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("clipmap_uniforms"),
            size: std::mem::size_of::<ClipmapUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&uniform_buffer, 0, bytemuck::bytes_of(&uniforms));

        // Sampler for heightmap
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("clipmap_heightmap_sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        // Bind group layout
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("clipmap_bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("clipmap_bg"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(heightmap_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        // Shader module
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("clipmap_terrain_shader"),
            source: wgpu::ShaderSource::Wgsl(CLIPMAP_TERRAIN_WGSL.into()),
        });

        // Pipeline layout
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("clipmap_pipeline_layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        // Render pipeline
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("clipmap_terrain_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<ClipmapVertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        // grid_pos: vec2<f32>
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x2,
                            offset: 0,
                            shader_location: 0,
                        },
                        // ring_info: vec4<f32>
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x4,
                            offset: 8,
                            shader_location: 1,
                        },
                    ],
                }],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: depth_format,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: Default::default(),
            multiview: None,
            cache: None,
        });

        Self {
            ring_vertex_buffers,
            ring_index_buffers,
            ring_index_counts,
            uniform_buffer,
            bind_group_layout,
            bind_group,
            pipeline,
            config: config.clone(),
        }
    }

    /// Update uniform buffer with current camera state.
    pub fn update(&self, queue: &wgpu::Queue, view_proj: glam::Mat4, camera_pos: glam::Vec3) {
        let uniforms = ClipmapUniforms {
            view_proj: view_proj.to_cols_array_2d(),
            camera_pos: camera_pos.to_array(),
            clipmap_scale: self.config.base_cell_size,
            morph_constants: [
                self.config.morph_start_distance(0),
                self.config.morph_range_inv(0),
                0.0,
                0.0,
            ],
            heightmap_size: [self.config.heightmap_width, self.config.heightmap_height],
            inv_heightmap_size: [
                1.0 / self.config.heightmap_width,
                1.0 / self.config.heightmap_height,
            ],
            heightmap_origin: [0.0; 2],
            _pad: [0.0; 2],
        };
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&uniforms));
    }

    /// Update uniform buffer for camera-relative rendering.
    ///
    /// `camera_pos` should be `Vec3::ZERO` (all geometry lives near the origin).
    /// `world_camera_xz` is the camera's true world XZ (from `DVec3`) downcast to
    /// `f32`, used so heightmap UV lookups reference the correct world location.
    pub fn update_camera_relative(
        &self,
        queue: &wgpu::Queue,
        view_proj: glam::Mat4,
        camera_pos: glam::Vec3,
        world_camera_xz: [f32; 2],
    ) {
        let uniforms = ClipmapUniforms {
            view_proj: view_proj.to_cols_array_2d(),
            camera_pos: camera_pos.to_array(),
            clipmap_scale: self.config.base_cell_size,
            morph_constants: [
                self.config.morph_start_distance(0),
                self.config.morph_range_inv(0),
                0.0,
                0.0,
            ],
            heightmap_size: [self.config.heightmap_width, self.config.heightmap_height],
            inv_heightmap_size: [
                1.0 / self.config.heightmap_width,
                1.0 / self.config.heightmap_height,
            ],
            heightmap_origin: world_camera_xz,
            _pad: [0.0; 2],
        };
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&uniforms));
    }

    /// Record draw commands for all clipmap rings into the given render pass.
    pub fn render<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>) {
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);

        for i in 0..self.ring_vertex_buffers.len() {
            if self.ring_index_counts[i] == 0 {
                continue;
            }
            pass.set_vertex_buffer(0, self.ring_vertex_buffers[i].slice(..));
            pass.set_index_buffer(
                self.ring_index_buffers[i].slice(..),
                wgpu::IndexFormat::Uint32,
            );
            pass.draw_indexed(0..self.ring_index_counts[i], 0, 0..1);
        }
    }

    /// Number of clipmap levels.
    pub fn num_levels(&self) -> u32 {
        self.config.num_levels
    }

    /// Total triangle count across all rings.
    pub fn total_triangles(&self) -> u32 {
        self.ring_index_counts.iter().sum::<u32>() / 3
    }

    /// Configuration reference.
    pub fn config(&self) -> &ClipmapConfig {
        &self.config
    }

    /// Bind group layout (for pipeline composition).
    pub fn bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.bind_group_layout
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clipmap_vertex_size() {
        assert_eq!(std::mem::size_of::<ClipmapVertex>(), 24);
    }

    #[test]
    fn clipmap_uniforms_size() {
        assert_eq!(std::mem::size_of::<ClipmapUniforms>(), 128);
    }

    #[test]
    fn default_config_valid() {
        let config = ClipmapConfig::default();
        assert!(config.validate().is_ok());
        assert_eq!(config.num_levels, 6);
        assert_eq!(config.grid_size, 63);
    }

    #[test]
    fn config_validation_rejects_invalid() {
        let mut config = ClipmapConfig::default();

        config.num_levels = 0;
        assert!(config.validate().is_err());

        config.num_levels = 6;
        config.grid_size = 4; // even
        assert!(config.validate().is_err());

        config.grid_size = 63;
        config.base_cell_size = -1.0;
        assert!(config.validate().is_err());

        config.base_cell_size = 1.0;
        config.morph_start_fraction = 1.5;
        assert!(config.validate().is_err());
    }

    #[test]
    fn cell_size_doubles_per_level() {
        let config = ClipmapConfig::default();
        assert!((config.cell_size(0) - 1.0).abs() < 1e-6);
        assert!((config.cell_size(1) - 2.0).abs() < 1e-6);
        assert!((config.cell_size(2) - 4.0).abs() < 1e-6);
        assert!((config.cell_size(3) - 8.0).abs() < 1e-6);
    }

    #[test]
    fn morph_distance_increases_with_level() {
        let config = ClipmapConfig::default();
        let d0 = config.morph_start_distance(0);
        let d1 = config.morph_start_distance(1);
        let d2 = config.morph_start_distance(2);
        assert!(d1 > d0);
        assert!(d2 > d1);
    }

    #[test]
    fn morph_range_inv_positive() {
        let config = ClipmapConfig::default();
        for level in 0..config.num_levels {
            let inv = config.morph_range_inv(level);
            assert!(
                inv > 0.0,
                "morph_range_inv should be positive at level {level}"
            );
        }
    }

    #[test]
    fn ring_mesh_generation_ring0_is_full_grid() {
        let config = ClipmapConfig {
            num_levels: 1,
            grid_size: 5,
            base_cell_size: 1.0,
            ..Default::default()
        };
        let rings = generate_clipmap_rings(&config);
        assert_eq!(rings.len(), 1);
        // 5×5 grid = 25 vertices
        assert_eq!(rings[0].vertices.len(), 25);
        // 4×4 quads × 2 triangles × 3 indices = 96
        assert_eq!(rings[0].indices.len(), 96);
    }

    #[test]
    fn ring_mesh_generation_ring1_has_hole() {
        let config = ClipmapConfig {
            num_levels: 2,
            grid_size: 7,
            base_cell_size: 1.0,
            ..Default::default()
        };
        let rings = generate_clipmap_rings(&config);
        assert_eq!(rings.len(), 2);
        // Ring 0: 7×7 = 49 vertices, full grid
        assert_eq!(rings[0].vertices.len(), 49);
        // Ring 1: should have fewer vertices (inner hole cut out)
        assert!(rings[1].vertices.len() < 49);
        assert!(rings[1].vertices.len() > 0);
        assert!(rings[1].indices.len() > 0);
    }

    #[test]
    fn ring_mesh_indices_in_bounds() {
        let config = ClipmapConfig {
            num_levels: 4,
            grid_size: 15,
            base_cell_size: 1.0,
            ..Default::default()
        };
        let rings = generate_clipmap_rings(&config);
        for ring in &rings {
            let max_idx = ring.vertices.len() as u32;
            for &idx in &ring.indices {
                assert!(
                    idx < max_idx,
                    "Index {} out of bounds (max {}) in ring {}",
                    idx,
                    max_idx,
                    ring.level
                );
            }
        }
    }

    #[test]
    fn ring_info_matches_level() {
        let config = ClipmapConfig {
            num_levels: 3,
            grid_size: 7,
            base_cell_size: 2.0,
            ..Default::default()
        };
        let rings = generate_clipmap_rings(&config);
        for ring in &rings {
            for v in &ring.vertices {
                assert_eq!(v.ring_info[0] as u32, ring.level);
                let expected_scale = (1u32 << ring.level) as f32;
                assert!((v.ring_info[1] - expected_scale).abs() < 1e-6);
            }
        }
    }

    #[test]
    fn parse_clipmap_wgsl() {
        let module = naga::front::wgsl::parse_str(CLIPMAP_TERRAIN_WGSL);
        assert!(
            module.is_ok(),
            "Failed to parse clipmap_terrain.wgsl: {:?}",
            module.err()
        );
    }

    #[test]
    fn clipmap_terrain_creation() {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let adapter =
            match pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::LowPower,
                ..Default::default()
            })) {
                Ok(a) => a,
                Err(_) => return,
            };
        let (device, queue) =
            match pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())) {
                Ok(dq) => dq,
                Err(_) => return,
            };

        // Create a dummy heightmap texture
        let heightmap = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("test_heightmap"),
            size: wgpu::Extent3d {
                width: 256,
                height: 256,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R16Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = heightmap.create_view(&Default::default());

        let config = ClipmapConfig {
            num_levels: 4,
            grid_size: 15,
            heightmap_width: 256.0,
            heightmap_height: 256.0,
            ..Default::default()
        };

        let clipmap = ClipmapTerrain::new(
            &device,
            &queue,
            &config,
            &view,
            wgpu::TextureFormat::Bgra8Unorm,
            wgpu::TextureFormat::Depth32Float,
        );

        assert_eq!(clipmap.num_levels(), 4);
        assert!(clipmap.total_triangles() > 0);
    }
}
