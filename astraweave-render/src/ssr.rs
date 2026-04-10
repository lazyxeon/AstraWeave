//! Screen-Space Reflections (SSR) with Hi-Z acceleration.
//!
//! Traces rays in screen space against the depth buffer to produce reflections.
//! Uses increasing step sizes and screen-edge fading for quality. Falls back
//! to IBL cubemap for misses.

/// GPU-side uniform parameters for the SSR compute pass.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SsrParams {
    pub inv_proj: [[f32; 4]; 4],
    pub proj: [[f32; 4]; 4],
    pub resolution: [f32; 2],
    pub inv_resolution: [f32; 2],
    pub max_distance: f32,
    pub stride: f32,
    pub max_steps: u32,
    pub thickness: f32,
    pub fade_start: f32,
    pub fade_end: f32,
    pub roughness_cutoff: f32,
    pub frame_index: u32,
}

impl Default for SsrParams {
    fn default() -> Self {
        Self {
            inv_proj: glam::Mat4::IDENTITY.to_cols_array_2d(),
            proj: glam::Mat4::IDENTITY.to_cols_array_2d(),
            resolution: [1920.0, 1080.0],
            inv_resolution: [1.0 / 1920.0, 1.0 / 1080.0],
            max_distance: 100.0,
            stride: 4.0,
            max_steps: 64,
            thickness: 0.3,
            fade_start: 0.1,
            fade_end: 0.02,
            roughness_cutoff: 0.5,
            frame_index: 0,
        }
    }
}

/// Configuration for the SSR system.
#[derive(Debug, Clone)]
pub struct SsrConfig {
    pub enabled: bool,
    pub max_steps: u32,
    pub stride: f32,
    pub thickness: f32,
    pub roughness_cutoff: f32,
}

impl Default for SsrConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_steps: 64,
            stride: 4.0,
            thickness: 0.3,
            roughness_cutoff: 0.5,
        }
    }
}

/// Manages SSR GPU resources.
pub struct SsrPass {
    config: SsrConfig,
    pipeline: wgpu::ComputePipeline,
    params_buf: wgpu::Buffer,
    #[allow(dead_code)] // texture must be kept alive for view to remain valid
    ssr_texture: wgpu::Texture,
    ssr_view: wgpu::TextureView,
    bgl: wgpu::BindGroupLayout,
    frame_index: u32,
    width: u32,
    height: u32,
    /// Cached sampler (static — never changes).
    sampler: wgpu::Sampler,
    /// Cached bind group (rebuilt on generation change).
    cached_bg: crate::bind_group_cache::CachedBindGroup,
}

impl SsrPass {
    pub fn new(device: &wgpu::Device, width: u32, height: u32) -> Self {
        let config = SsrConfig::default();
        let fmt = wgpu::TextureFormat::Rgba16Float;

        let ssr_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("ssr_output"),
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
        });
        let ssr_view = ssr_texture.create_view(&wgpu::TextureViewDescriptor::default());

        use wgpu::util::DeviceExt;
        let params = SsrParams::default();
        let params_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("ssr_params"),
            contents: bytemuck::bytes_of(&params),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("ssr_bgl"),
            entries: &[
                tex_entry(0),          // depth
                tex_entry(1),          // normals
                tex_entry(2),          // color
                tex_entry(3),          // metallic-roughness
                sampler_entry(4),      // sampler
                uniform_entry(5),      // params
                storage_entry(6, fmt), // output
            ],
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("ssr_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/ssr.wgsl").into()),
        });
        let pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("ssr_pl"),
            bind_group_layouts: &[&bgl],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("ssr_pipeline"),
            layout: Some(&pl),
            module: &shader,
            entry_point: Some("ssr_main"),
            compilation_options: Default::default(),
            cache: None,
        });

        Self {
            config,
            pipeline,
            params_buf,
            ssr_texture,
            ssr_view,
            bgl,
            frame_index: 0,
            width,
            height,
            sampler: device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some("ssr_sampler"),
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                ..Default::default()
            }),
            cached_bg: crate::bind_group_cache::CachedBindGroup::new(),
        }
    }

    pub fn reflection_view(&self) -> &wgpu::TextureView {
        &self.ssr_view
    }

    pub fn config(&self) -> &SsrConfig {
        &self.config
    }

    pub fn set_config(&mut self, config: SsrConfig) {
        self.config = config;
    }

    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    pub fn update_params(&mut self, queue: &wgpu::Queue, proj: glam::Mat4) {
        let params = SsrParams {
            inv_proj: proj.inverse().to_cols_array_2d(),
            proj: proj.to_cols_array_2d(),
            resolution: [self.width as f32, self.height as f32],
            inv_resolution: [1.0 / self.width as f32, 1.0 / self.height as f32],
            max_distance: 100.0,
            stride: self.config.stride,
            max_steps: self.config.max_steps,
            thickness: self.config.thickness,
            fade_start: 0.1,
            fade_end: 0.02,
            roughness_cutoff: self.config.roughness_cutoff,
            frame_index: self.frame_index,
        };
        queue.write_buffer(&self.params_buf, 0, bytemuck::bytes_of(&params));
        self.frame_index = self.frame_index.wrapping_add(1);
    }

    #[allow(clippy::too_many_arguments)]
    pub fn execute(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        depth_view: &wgpu::TextureView,
        normal_view: &wgpu::TextureView,
        color_view: &wgpu::TextureView,
        mr_view: &wgpu::TextureView,
        resource_gen: crate::bind_group_cache::Generation,
    ) {
        if !self.config.enabled {
            return;
        }

        if !self.cached_bg.is_valid(resource_gen) {
            let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("ssr_bg"),
                layout: &self.bgl,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(depth_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(normal_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(color_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::TextureView(mr_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 5,
                        resource: self.params_buf.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 6,
                        resource: wgpu::BindingResource::TextureView(&self.ssr_view),
                    },
                ],
            });
            self.cached_bg =
                crate::bind_group_cache::CachedBindGroup::with_bind_group(bg, resource_gen);
        }

        let wg_x = self.width.div_ceil(8);
        let wg_y = self.height.div_ceil(8);

        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("ssr"),
            timestamp_writes: None,
        });
        pass.set_pipeline(&self.pipeline);
        let bg = self
            .cached_bg
            .get_or_rebuild(resource_gen, || unreachable!());
        pass.set_bind_group(0, bg, &[]);
        pass.dispatch_workgroups(wg_x, wg_y, 1);
    }

    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        if self.width == width && self.height == height {
            return;
        }
        *self = Self::new(device, width, height);
    }
}

fn tex_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
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

fn sampler_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
        count: None,
    }
}

fn uniform_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
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

fn storage_entry(binding: u32, format: wgpu::TextureFormat) -> wgpu::BindGroupLayoutEntry {
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
    fn ssr_params_size() {
        assert_eq!(std::mem::size_of::<SsrParams>(), 176);
    }

    #[test]
    fn ssr_config_default() {
        let c = SsrConfig::default();
        assert!(c.enabled);
        assert_eq!(c.max_steps, 64);
    }

    #[test]
    fn ssr_pass_creation() {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower,
            compatible_surface: None,
            force_fallback_adapter: false,
        }))
        .expect("adapter");
        let (device, _queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
                .expect("device");

        let pass = SsrPass::new(&device, 1920, 1080);
        assert_eq!(pass.dimensions(), (1920, 1080));
    }
}
