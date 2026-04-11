//! Lumen Final Gather — multi-bounce diffuse indirect lighting compositor.
//!
//! Combines three GI sources into a unified indirect diffuse result:
//!
//! 1. **Screen-space GI (SSGI)** — near-field, high-detail indirect
//! 2. **Surface cache probes (SH)** — far-field, multi-bounce irradiance
//! 3. **Distance-field AO (DFAO)** — long-range occlusion modulation
//!
//! The final gather applies temporal reprojection with neighborhood clamping
//! for flicker-free output.

use bytemuck::{Pod, Zeroable};
use glam::{Mat4, UVec3};
use wgpu::util::DeviceExt;

// ---------------------------------------------------------------------------
// GPU types
// ---------------------------------------------------------------------------

/// GPU-side uniform parameters for the final gather compute pass.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct FinalGatherParams {
    pub inv_view_proj: [[f32; 4]; 4],
    pub prev_view_proj: [[f32; 4]; 4],
    pub resolution: [f32; 2],
    pub inv_resolution: [f32; 2],
    pub grid_origin: [f32; 3],
    pub probe_spacing: f32,
    pub grid_dims: [u32; 3],
    pub ssgi_weight: f32,
    pub probe_weight: f32,
    pub dfao_weight: f32,
    pub temporal_blend: f32,
    pub frame_index: u32,
    pub near_plane: f32,
    pub far_plane: f32,
    pub _pad0: f32,
    pub _pad1: f32,
}

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Configuration for the Lumen final gather pass.
#[derive(Debug, Clone)]
pub struct FinalGatherConfig {
    pub enabled: bool,
    /// Weight for screen-space GI contribution (0.0–1.0).
    pub ssgi_weight: f32,
    /// Weight for probe irradiance contribution (0.0–1.0).
    pub probe_weight: f32,
    /// Weight for DFAO occlusion modulation (0.0–1.0).
    pub dfao_weight: f32,
    /// Temporal blend factor (higher = more history, more stable).
    pub temporal_blend: f32,
}

impl Default for FinalGatherConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            ssgi_weight: 0.6,
            probe_weight: 0.4,
            dfao_weight: 0.8,
            temporal_blend: 0.9,
        }
    }
}

// ---------------------------------------------------------------------------
// Final Gather Pass
// ---------------------------------------------------------------------------

/// Manages GPU resources for the Lumen final gather composite pass.
pub struct FinalGatherPass {
    config: FinalGatherConfig,
    pipeline: wgpu::ComputePipeline,
    bgl: wgpu::BindGroupLayout,
    params_buf: wgpu::Buffer,
    /// Current frame output.
    output_texture: wgpu::Texture,
    output_view: wgpu::TextureView,
    /// History buffer for temporal reprojection.
    history_texture: wgpu::Texture,
    history_view: wgpu::TextureView,
    frame_index: u32,
    prev_view_proj: Mat4,
    width: u32,
    height: u32,
    /// Cached linear sampler (reused across frames).
    sampler: wgpu::Sampler,
    /// Cached bind group (generation-tracked).
    cached_bg: crate::bind_group_cache::CachedBindGroup,
}

impl FinalGatherPass {
    pub fn new(device: &wgpu::Device, width: u32, height: u32) -> Self {
        let config = FinalGatherConfig::default();
        let fmt = wgpu::TextureFormat::Rgba16Float;

        let make_tex = |label: &str| {
            device.create_texture(&wgpu::TextureDescriptor {
                label: Some(label),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: fmt,
                usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            })
        };

        let output_texture = make_tex("final_gather_output");
        let output_view = output_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let history_texture = make_tex("final_gather_history");
        let history_view = history_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let params = FinalGatherParams {
            inv_view_proj: Mat4::IDENTITY.to_cols_array_2d(),
            prev_view_proj: Mat4::IDENTITY.to_cols_array_2d(),
            resolution: [width as f32, height as f32],
            inv_resolution: [1.0 / width as f32, 1.0 / height as f32],
            grid_origin: [0.0; 3],
            probe_spacing: 4.0,
            grid_dims: [16, 8, 16],
            ssgi_weight: config.ssgi_weight,
            probe_weight: config.probe_weight,
            dfao_weight: config.dfao_weight,
            temporal_blend: config.temporal_blend,
            frame_index: 0,
            near_plane: 0.1,
            far_plane: 1000.0,
            _pad0: 0.0,
            _pad1: 0.0,
        };
        let params_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("final_gather_params"),
            contents: bytemuck::bytes_of(&params),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("final_gather_bgl"),
            entries: &[
                bgl_uniform(0),              // params
                bgl_texture_2d(1),           // depth
                bgl_texture_2d(2),           // normal
                bgl_texture_2d(3),           // albedo
                bgl_texture_2d(4),           // ssgi
                bgl_texture_2d(5),           // dfao
                bgl_texture_2d(6),           // velocity
                bgl_texture_2d(7),           // history
                bgl_storage_ro(8),           // probes
                bgl_sampler(9),              // sampler
                bgl_storage_tex_rw(10, fmt), // output
            ],
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("final_gather_shader"),
            source: wgpu::ShaderSource::Wgsl(
                concat!(include_str!("../shaders/constants.wgsl"),
                include_str!("../shaders/lumen/final_gather.wgsl")).into(),
            ),
        });
        let pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("final_gather_pl"),
            bind_group_layouts: &[&bgl],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("final_gather_pipeline"),
            layout: Some(&pl),
            module: &shader,
            entry_point: Some("final_gather_main"),
            compilation_options: Default::default(),
            cache: None,
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("final_gather_sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        Self {
            config,
            pipeline,
            bgl,
            params_buf,
            output_texture,
            output_view,
            history_texture,
            history_view,
            frame_index: 0,
            prev_view_proj: Mat4::IDENTITY,
            width,
            height,
            sampler,
            cached_bg: crate::bind_group_cache::CachedBindGroup::new(),
        }
    }

    /// Get the final indirect lighting output view.
    pub fn output_view(&self) -> &wgpu::TextureView {
        &self.output_view
    }

    pub fn config(&self) -> &FinalGatherConfig {
        &self.config
    }

    pub fn set_config(&mut self, config: FinalGatherConfig) {
        self.config = config;
    }

    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Update parameters for this frame.
    #[allow(clippy::too_many_arguments)]
    pub fn update_params(
        &mut self,
        queue: &wgpu::Queue,
        inv_view_proj: Mat4,
        grid_origin: [f32; 3],
        probe_spacing: f32,
        grid_dims: UVec3,
        near: f32,
        far: f32,
    ) {
        let params = FinalGatherParams {
            inv_view_proj: inv_view_proj.to_cols_array_2d(),
            prev_view_proj: self.prev_view_proj.to_cols_array_2d(),
            resolution: [self.width as f32, self.height as f32],
            inv_resolution: [1.0 / self.width as f32, 1.0 / self.height as f32],
            grid_origin,
            probe_spacing,
            grid_dims: grid_dims.to_array(),
            ssgi_weight: self.config.ssgi_weight,
            probe_weight: self.config.probe_weight,
            dfao_weight: self.config.dfao_weight,
            temporal_blend: self.config.temporal_blend,
            frame_index: self.frame_index,
            near_plane: near,
            far_plane: far,
            _pad0: 0.0,
            _pad1: 0.0,
        };
        queue.write_buffer(&self.params_buf, 0, bytemuck::bytes_of(&params));

        // Store current VP for next frame's temporal reprojection
        // (caller should pass view_proj, not inv — fix up caller side)
        self.prev_view_proj = inv_view_proj.inverse();
        self.frame_index = self.frame_index.wrapping_add(1);
    }

    /// Dispatch the final gather compute pass.
    #[allow(clippy::too_many_arguments)]
    pub fn execute(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        depth_view: &wgpu::TextureView,
        normal_view: &wgpu::TextureView,
        albedo_view: &wgpu::TextureView,
        ssgi_view: &wgpu::TextureView,
        dfao_view: &wgpu::TextureView,
        velocity_view: &wgpu::TextureView,
        probe_buffer: &wgpu::Buffer,
        resource_gen: crate::bind_group_cache::Generation,
    ) {
        if !self.config.enabled {
            return;
        }

        let bg = self.cached_bg.get_or_rebuild(resource_gen, || {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("final_gather_bg"),
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
                        resource: wgpu::BindingResource::TextureView(albedo_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: wgpu::BindingResource::TextureView(ssgi_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 5,
                        resource: wgpu::BindingResource::TextureView(dfao_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 6,
                        resource: wgpu::BindingResource::TextureView(velocity_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 7,
                        resource: wgpu::BindingResource::TextureView(&self.history_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 8,
                        resource: probe_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 9,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 10,
                        resource: wgpu::BindingResource::TextureView(&self.output_view),
                    },
                ],
            })
        });

        let wg_x = self.width.div_ceil(8);
        let wg_y = self.height.div_ceil(8);

        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("final_gather"),
            timestamp_writes: None,
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, bg, &[]);
        pass.dispatch_workgroups(wg_x, wg_y, 1);
    }

    /// Copy output to history for next frame's temporal reprojection.
    pub fn copy_to_history(&self, encoder: &mut wgpu::CommandEncoder) {
        encoder.copy_texture_to_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.output_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyTextureInfo {
                texture: &self.history_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
        );
    }

    /// Resize output/history textures.
    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        if self.width == width && self.height == height {
            return;
        }
        *self = Self::new(device, width, height);
    }
}

// ---------------------------------------------------------------------------
// Bind group layout helpers
// ---------------------------------------------------------------------------

fn bgl_uniform(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

fn bgl_texture_2d(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: true },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        },
        count: None,
    }
}

fn bgl_storage_ro(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only: true },
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

fn bgl_sampler(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
        count: None,
    }
}

fn bgl_storage_tex_rw(binding: u32, format: wgpu::TextureFormat) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::StorageTexture {
            access: wgpu::StorageTextureAccess::WriteOnly,
            format,
            view_dimension: wgpu::TextureViewDimension::D2,
        },
        count: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn final_gather_params_size() {
        // 2 mat4x4 (128) + 4 floats (16) + 3 floats + pad (16) + 3 u32 + pad (16) +
        // 4 floats (16) + 4 floats (16) = 208
        assert_eq!(std::mem::size_of::<FinalGatherParams>(), 208);
    }

    #[test]
    fn default_config() {
        let c = FinalGatherConfig::default();
        assert!(c.enabled);
        assert!((c.ssgi_weight - 0.6).abs() < 1e-6);
        assert!((c.probe_weight - 0.4).abs() < 1e-6);
        assert!((c.dfao_weight - 0.8).abs() < 1e-6);
        assert!((c.temporal_blend - 0.9).abs() < 1e-6);
    }

    #[test]
    fn weights_sum_reasonable() {
        let c = FinalGatherConfig::default();
        let total = c.ssgi_weight + c.probe_weight;
        assert!(
            total > 0.5 && total <= 1.5,
            "GI weights should sum to ~1.0: {total}"
        );
    }

    #[test]
    fn final_gather_pass_creation() {
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

        let pass = FinalGatherPass::new(&device, 1280, 720);
        assert_eq!(pass.dimensions(), (1280, 720));
    }
}
