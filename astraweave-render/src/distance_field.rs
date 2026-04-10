//! Distance Field Ambient Occlusion (DFAO) for Lumen GI.
//!
//! Generates a 3D signed distance field (SDF) from scene geometry and provides
//! a compute-based cone-traced AO pass that extends far beyond screen-space methods.
//! The SDF is stored in a 3D texture and updated incrementally.
//!
//! DFAO produces soft, physically-plausible occlusion with infinite range — perfect
//! for large-scale architectural occlusion that GTAO/SSAO cannot capture.

use bytemuck::{Pod, Zeroable};
use glam::Vec3;
use wgpu::util::DeviceExt;

// ---------------------------------------------------------------------------
// GPU types
// ---------------------------------------------------------------------------

/// GPU-side uniform parameters for the DFAO compute pass.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct DfaoParams {
    pub inv_view_proj: [[f32; 4]; 4],
    pub view_pos: [f32; 3],
    pub max_distance: f32,
    pub resolution: [f32; 2],
    pub inv_resolution: [f32; 2],
    pub sdf_origin: [f32; 3],
    pub sdf_inv_size: f32,
    pub sdf_dims: [u32; 3],
    pub num_steps: u32,
    pub ao_power: f32,
    pub ao_intensity: f32,
    pub near_plane: f32,
    pub far_plane: f32,
}

// ---------------------------------------------------------------------------
// SDF Volume
// ---------------------------------------------------------------------------

/// A bounding box for SDF generation.
#[derive(Debug, Clone, Copy)]
pub struct SdfBox {
    pub center: Vec3,
    pub half_extents: Vec3,
}

/// Configuration for the SDF volume.
#[derive(Debug, Clone)]
pub struct SdfConfig {
    /// Resolution of the 3D SDF texture (per axis).
    pub dims: [u32; 3],
    /// World-space origin (min corner).
    pub origin: Vec3,
    /// World-space size of the entire volume.
    pub world_size: f32,
}

impl Default for SdfConfig {
    fn default() -> Self {
        Self {
            dims: [64, 32, 64],
            origin: Vec3::new(-64.0, -16.0, -64.0),
            world_size: 128.0,
        }
    }
}

impl SdfConfig {
    /// Voxel size in world units.
    pub fn voxel_size(&self) -> f32 {
        self.world_size / self.dims[0].max(self.dims[1]).max(self.dims[2]) as f32
    }
}

/// CPU-side SDF volume for baking distance fields from geometry.
pub struct SdfVolume {
    config: SdfConfig,
    /// Distance values: positive = outside, negative = inside.
    data: Vec<f32>,
}

impl SdfVolume {
    /// Create a new SDF volume initialized to max distance.
    pub fn new(config: SdfConfig) -> Self {
        let total = config.dims[0] as usize * config.dims[1] as usize * config.dims[2] as usize;
        Self {
            config,
            data: vec![f32::MAX; total],
        }
    }

    /// Total voxel count.
    pub fn total_voxels(&self) -> usize {
        self.data.len()
    }

    /// Get the SDF value at a voxel index.
    pub fn get(&self, x: u32, y: u32, z: u32) -> f32 {
        let idx = self.linear_index(x, y, z);
        self.data[idx]
    }

    /// World position of a voxel center.
    pub fn voxel_center(&self, x: u32, y: u32, z: u32) -> Vec3 {
        let voxel_size = Vec3::new(
            self.config.world_size / self.config.dims[0] as f32,
            self.config.world_size / self.config.dims[1] as f32,
            self.config.world_size / self.config.dims[2] as f32,
        );
        self.config.origin + Vec3::new(x as f32 + 0.5, y as f32 + 0.5, z as f32 + 0.5) * voxel_size
    }

    fn linear_index(&self, x: u32, y: u32, z: u32) -> usize {
        (z as usize * self.config.dims[1] as usize + y as usize) * self.config.dims[0] as usize
            + x as usize
    }

    /// Bake SDF from a list of axis-aligned bounding boxes.
    /// Uses brute-force signed distance computation: for each voxel, find the
    /// minimum distance to any box surface.
    pub fn bake_from_boxes(&mut self, boxes: &[SdfBox]) {
        let [dx, dy, dz] = self.config.dims;
        for z in 0..dz {
            for y in 0..dy {
                for x in 0..dx {
                    let pos = self.voxel_center(x, y, z);
                    let mut min_dist = f32::MAX;

                    for b in boxes {
                        let d = sdf_box(pos, b.center, b.half_extents);
                        min_dist = min_dist.min(d);
                    }

                    let idx = self.linear_index(x, y, z);
                    self.data[idx] = min_dist;
                }
            }
        }
    }

    /// Raw float data for GPU upload.
    pub fn data(&self) -> &[f32] {
        &self.data
    }

    pub fn config(&self) -> &SdfConfig {
        &self.config
    }
}

/// Signed distance from point `p` to an axis-aligned box at `center` with `half_extents`.
fn sdf_box(p: Vec3, center: Vec3, half_extents: Vec3) -> f32 {
    let q = (p - center).abs() - half_extents;
    let outside = Vec3::new(q.x.max(0.0), q.y.max(0.0), q.z.max(0.0)).length();
    let inside = q.x.max(q.y.max(q.z)).min(0.0);
    outside + inside
}

// ---------------------------------------------------------------------------
// DFAO Configuration
// ---------------------------------------------------------------------------

/// Configuration for the DFAO pass.
#[derive(Debug, Clone)]
pub struct DfaoConfig {
    pub enabled: bool,
    /// Maximum trace distance in world units.
    pub max_distance: f32,
    /// Number of sphere-trace iterations.
    pub num_steps: u32,
    /// AO contrast exponent (higher = darker, more contrasty).
    pub ao_power: f32,
    /// AO intensity multiplier.
    pub ao_intensity: f32,
}

impl Default for DfaoConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_distance: 30.0,
            num_steps: 12,
            ao_power: 1.5,
            ao_intensity: 1.0,
        }
    }
}

// ---------------------------------------------------------------------------
// DFAO Pass
// ---------------------------------------------------------------------------

/// Manages GPU resources for the DFAO compute pass.
pub struct DfaoPass {
    config: DfaoConfig,
    sdf_config: SdfConfig,
    pipeline: wgpu::ComputePipeline,
    bgl: wgpu::BindGroupLayout,
    params_buf: wgpu::Buffer,
    /// 3D SDF texture.
    sdf_texture: wgpu::Texture,
    sdf_view: wgpu::TextureView,
    /// Output AO texture (R16Float).
    ao_texture: wgpu::Texture,
    ao_view: wgpu::TextureView,
    width: u32,
    height: u32,
    /// Cached linear+clamp sampler (reused across frames).
    sampler: wgpu::Sampler,
    /// Cached bind group (generation-tracked).
    cached_bg: crate::bind_group_cache::CachedBindGroup,
}

impl DfaoPass {
    pub fn new(device: &wgpu::Device, width: u32, height: u32, sdf_config: SdfConfig) -> Self {
        let config = DfaoConfig::default();
        let [sx, sy, sz] = sdf_config.dims;

        // Create 3D SDF texture
        let sdf_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("sdf_volume"),
            size: wgpu::Extent3d {
                width: sx,
                height: sy,
                depth_or_array_layers: sz,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D3,
            format: wgpu::TextureFormat::R32Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let sdf_view = sdf_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // AO output
        let ao_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("dfao_output"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba16Float,
            usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let ao_view = ao_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let params = DfaoParams {
            inv_view_proj: glam::Mat4::IDENTITY.to_cols_array_2d(),
            view_pos: [0.0; 3],
            max_distance: config.max_distance,
            resolution: [width as f32, height as f32],
            inv_resolution: [1.0 / width as f32, 1.0 / height as f32],
            sdf_origin: sdf_config.origin.to_array(),
            sdf_inv_size: 1.0 / sdf_config.world_size,
            sdf_dims: sdf_config.dims,
            num_steps: config.num_steps,
            ao_power: config.ao_power,
            ao_intensity: config.ao_intensity,
            near_plane: 0.1,
            far_plane: 1000.0,
        };
        let params_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("dfao_params"),
            contents: bytemuck::bytes_of(&params),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("dfao_bgl"),
            entries: &[
                // 0: params
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
                // 1: depth
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
                // 2: normals
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                // 3: SDF 3D texture
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D3,
                        multisampled: false,
                    },
                    count: None,
                },
                // 4: sampler
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                // 5: AO output (storage)
                wgpu::BindGroupLayoutEntry {
                    binding: 5,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::WriteOnly,
                        format: wgpu::TextureFormat::Rgba16Float,
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                },
            ],
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("dfao_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/lumen/dfao.wgsl").into()),
        });
        let pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("dfao_pl"),
            bind_group_layouts: &[&bgl],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("dfao_pipeline"),
            layout: Some(&pl),
            module: &shader,
            entry_point: Some("dfao_main"),
            compilation_options: Default::default(),
            cache: None,
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("dfao_sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            ..Default::default()
        });

        Self {
            config,
            sdf_config,
            pipeline,
            bgl,
            params_buf,
            sdf_texture,
            sdf_view,
            ao_texture,
            ao_view,
            width,
            height,
            sampler,
            cached_bg: crate::bind_group_cache::CachedBindGroup::new(),
        }
    }

    /// Get the AO output texture view.
    pub fn ao_view(&self) -> &wgpu::TextureView {
        &self.ao_view
    }

    pub fn config(&self) -> &DfaoConfig {
        &self.config
    }

    pub fn set_config(&mut self, config: DfaoConfig) {
        self.config = config;
    }

    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Upload SDF volume data to the GPU.
    pub fn upload_sdf(&self, queue: &wgpu::Queue, volume: &SdfVolume) {
        let [sx, sy, sz] = volume.config().dims;
        let bytes_per_row = sx * 4; // R32Float = 4 bytes per texel

        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.sdf_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            bytemuck::cast_slice(volume.data()),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(bytes_per_row),
                rows_per_image: Some(sy),
            },
            wgpu::Extent3d {
                width: sx,
                height: sy,
                depth_or_array_layers: sz,
            },
        );
    }

    /// Update params for this frame.
    pub fn update_params(
        &self,
        queue: &wgpu::Queue,
        inv_view_proj: glam::Mat4,
        view_pos: Vec3,
        near: f32,
        far: f32,
    ) {
        let params = DfaoParams {
            inv_view_proj: inv_view_proj.to_cols_array_2d(),
            view_pos: view_pos.to_array(),
            max_distance: self.config.max_distance,
            resolution: [self.width as f32, self.height as f32],
            inv_resolution: [1.0 / self.width as f32, 1.0 / self.height as f32],
            sdf_origin: self.sdf_config.origin.to_array(),
            sdf_inv_size: 1.0 / self.sdf_config.world_size,
            sdf_dims: self.sdf_config.dims,
            num_steps: self.config.num_steps,
            ao_power: self.config.ao_power,
            ao_intensity: self.config.ao_intensity,
            near_plane: near,
            far_plane: far,
        };
        queue.write_buffer(&self.params_buf, 0, bytemuck::bytes_of(&params));
    }

    /// Dispatch the DFAO compute pass.
    pub fn execute(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        depth_view: &wgpu::TextureView,
        normal_view: &wgpu::TextureView,
        resource_gen: crate::bind_group_cache::Generation,
    ) {
        if !self.config.enabled {
            return;
        }

        let bg = self.cached_bg.get_or_rebuild(resource_gen, || {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("dfao_bg"),
                layout: &self.bgl,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: self.params_buf.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(depth_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(normal_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::TextureView(&self.sdf_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 5,
                        resource: wgpu::BindingResource::TextureView(&self.ao_view),
                    },
                ],
            })
        });

        let wg_x = (self.width + 7) / 8;
        let wg_y = (self.height + 7) / 8;

        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("dfao"),
            timestamp_writes: None,
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, bg, &[]);
        pass.dispatch_workgroups(wg_x, wg_y, 1);
    }

    /// Resize AO output texture.
    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        if self.width == width && self.height == height {
            return;
        }
        let sdf_config = self.sdf_config.clone();
        *self = Self::new(device, width, height, sdf_config);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dfao_params_size() {
        // mat4x4 (64) + vec3+pad (16) + vec2+vec2 (16) + vec3+pad (16) +
        // uvec3+pad (16) + 4 floats (16) = 144
        assert_eq!(std::mem::size_of::<DfaoParams>(), 144);
    }

    #[test]
    fn sdf_box_distance() {
        // Point at origin, box centered at origin with half-extents 1
        let d = sdf_box(Vec3::ZERO, Vec3::ZERO, Vec3::ONE);
        assert!(d < 0.0, "Point inside box should be negative: {d}");

        // Point outside the box
        let d2 = sdf_box(Vec3::new(3.0, 0.0, 0.0), Vec3::ZERO, Vec3::ONE);
        assert!((d2 - 2.0).abs() < 1e-5, "Expected ~2.0, got {d2}");

        // Point on surface
        let d3 = sdf_box(Vec3::new(1.0, 0.0, 0.0), Vec3::ZERO, Vec3::ONE);
        assert!(d3.abs() < 1e-5, "Surface should be ~0.0, got {d3}");
    }

    #[test]
    fn sdf_box_corner() {
        // Point at corner (2,2,2), box at origin with half-extents 1
        let d = sdf_box(Vec3::new(2.0, 2.0, 2.0), Vec3::ZERO, Vec3::ONE);
        let expected = Vec3::ONE.length(); // sqrt(3)
        assert!((d - expected).abs() < 1e-5);
    }

    #[test]
    fn sdf_volume_creation() {
        let config = SdfConfig {
            dims: [4, 4, 4],
            origin: Vec3::ZERO,
            world_size: 4.0,
        };
        let vol = SdfVolume::new(config);
        assert_eq!(vol.total_voxels(), 64);
        assert_eq!(vol.get(0, 0, 0), f32::MAX);
    }

    #[test]
    fn sdf_volume_bake() {
        let config = SdfConfig {
            dims: [8, 8, 8],
            origin: Vec3::splat(-4.0),
            world_size: 8.0,
        };
        let mut vol = SdfVolume::new(config);

        let boxes = vec![SdfBox {
            center: Vec3::ZERO,
            half_extents: Vec3::ONE,
        }];
        vol.bake_from_boxes(&boxes);

        // Center voxel should be inside (negative)
        let center = vol.get(4, 4, 4);
        assert!(center < 0.0, "Center should be inside: {center}");

        // Far corner should be positive
        let corner = vol.get(0, 0, 0);
        assert!(corner > 0.0, "Corner should be outside: {corner}");
    }

    #[test]
    fn sdf_voxel_size() {
        let config = SdfConfig {
            dims: [64, 32, 64],
            origin: Vec3::ZERO,
            world_size: 128.0,
        };
        assert!((config.voxel_size() - 2.0).abs() < 1e-5);
    }

    #[test]
    fn dfao_pass_creation() {
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

        let sdf = SdfConfig {
            dims: [8, 8, 8],
            ..Default::default()
        };
        let pass = DfaoPass::new(&device, 640, 480, sdf);
        assert_eq!(pass.dimensions(), (640, 480));
    }

    #[test]
    fn default_config() {
        let c = DfaoConfig::default();
        assert!(c.enabled);
        assert_eq!(c.num_steps, 12);
        assert!((c.max_distance - 30.0).abs() < 1e-6);
    }
}
