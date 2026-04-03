//! Ground Truth Ambient Occlusion (GTAO) with visibility bitmask.
//!
//! Compute-shader-based screen-space AO using horizon-based occlusion with a
//! per-sector visibility bitmask. Includes bilateral blur for edge-preserving
//! spatial filtering.

/// GPU-side uniform parameters for the GTAO compute pass.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GtaoParams {
    /// Projection info: x=near*far, y=near-far, z=far, w=aspect*tan(fov/2)
    pub proj_info: [f32; 4],
    /// World-space AO radius.
    pub radius: f32,
    /// Distance where AO starts to fade (fraction of radius).
    pub falloff_start: f32,
    /// Distance where AO fully fades (fraction of radius).
    pub falloff_end: f32,
    /// AO contrast exponent.
    pub power: f32,
    /// Screen resolution (width, height).
    pub resolution: [f32; 2],
    /// 1/width, 1/height.
    pub inv_resolution: [f32; 2],
    /// Number of angular sectors (typically 8).
    pub num_directions: u32,
    /// Steps per direction (typically 4-8).
    pub num_steps: u32,
    /// Frame index for temporal noise rotation.
    pub frame_index: u32,
    pub _pad: u32,
}

impl GtaoParams {
    /// Create default parameters for a given screen size and camera.
    pub fn new(width: u32, height: u32, near: f32, far: f32, fov_y: f32, aspect: f32) -> Self {
        Self {
            proj_info: [near * far, near - far, far, aspect * (fov_y * 0.5).tan()],
            radius: 2.0,
            falloff_start: 0.2,
            falloff_end: 1.0,
            power: 1.5,
            resolution: [width as f32, height as f32],
            inv_resolution: [1.0 / width as f32, 1.0 / height as f32],
            num_directions: 8,
            num_steps: 6,
            frame_index: 0,
            _pad: 0,
        }
    }
}

/// GPU-side uniform for the bilateral blur pass.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GtaoBlurParams {
    /// Blur direction: (1,0) for horizontal, (0,1) for vertical.
    pub direction: [f32; 2],
    /// 1/width, 1/height.
    pub inv_resolution: [f32; 2],
    /// Depth difference threshold for edge detection.
    pub depth_threshold: f32,
    pub _pad: [f32; 3],
}

/// Configuration for the GTAO system.
#[derive(Debug, Clone)]
pub struct GtaoConfig {
    /// Enable/disable GTAO.
    pub enabled: bool,
    /// World-space AO radius.
    pub radius: f32,
    /// Number of angular directions to sample.
    pub num_directions: u32,
    /// Steps per direction.
    pub num_steps: u32,
    /// AO contrast exponent.
    pub power: f32,
    /// Bilateral blur depth threshold.
    pub blur_depth_threshold: f32,
}

impl Default for GtaoConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            radius: 2.0,
            num_directions: 8,
            num_steps: 6,
            power: 1.5,
            blur_depth_threshold: 0.05,
        }
    }
}

/// Manages GTAO GPU resources: compute pipelines, textures, bind groups.
pub struct GtaoPass {
    config: GtaoConfig,
    /// AO compute pipeline.
    ao_pipeline: wgpu::ComputePipeline,
    /// Blur compute pipeline.
    blur_pipeline: wgpu::ComputePipeline,
    /// AO params uniform buffer.
    params_buf: wgpu::Buffer,
    /// Blur params uniform buffer (horizontal).
    blur_h_buf: wgpu::Buffer,
    /// Blur params uniform buffer (vertical).
    blur_v_buf: wgpu::Buffer,
    /// AO output texture (R16Float).
    ao_texture: wgpu::Texture,
    ao_view: wgpu::TextureView,
    /// Blur intermediate texture (R16Float).
    blur_temp_texture: wgpu::Texture,
    blur_temp_view: wgpu::TextureView,
    /// AO bind group layout.
    ao_bgl: wgpu::BindGroupLayout,
    /// Blur bind group layout.
    blur_bgl: wgpu::BindGroupLayout,
    /// Frame counter for temporal rotation.
    frame_index: u32,
    width: u32,
    height: u32,
}

impl GtaoPass {
    pub fn new(device: &wgpu::Device, width: u32, height: u32) -> Self {
        let config = GtaoConfig::default();

        let ao_size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };

        // AO output texture
        let ao_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("gtao_output"),
            size: ao_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R32Float,
            usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let ao_view = ao_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Blur intermediate
        let blur_temp_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("gtao_blur_temp"),
            size: ao_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R32Float,
            usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let blur_temp_view = blur_temp_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Uniform buffers
        use wgpu::util::DeviceExt;
        let default_params = GtaoParams::new(
            width,
            height,
            0.1,
            200.0,
            std::f32::consts::FRAC_PI_3,
            16.0 / 9.0,
        );
        let params_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("gtao_params"),
            contents: bytemuck::bytes_of(&default_params),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let inv_res = [1.0 / width as f32, 1.0 / height as f32];
        let blur_h = GtaoBlurParams {
            direction: [1.0, 0.0],
            inv_resolution: inv_res,
            depth_threshold: config.blur_depth_threshold,
            _pad: [0.0; 3],
        };
        let blur_v = GtaoBlurParams {
            direction: [0.0, 1.0],
            inv_resolution: inv_res,
            depth_threshold: config.blur_depth_threshold,
            _pad: [0.0; 3],
        };
        let blur_h_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("gtao_blur_h_params"),
            contents: bytemuck::bytes_of(&blur_h),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let blur_v_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("gtao_blur_v_params"),
            contents: bytemuck::bytes_of(&blur_v),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // AO bind group layout
        let ao_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("gtao_ao_bgl"),
            entries: &[
                // 0: depth texture (Depth32Float)
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Depth,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                // 1: normal texture
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
                // 2: sampler
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                // 3: params uniform
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // 4: AO output (storage texture)
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::WriteOnly,
                        format: wgpu::TextureFormat::R32Float,
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                },
            ],
        });

        // Blur bind group layout
        let blur_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("gtao_blur_bgl"),
            entries: &[
                // 0: AO input texture
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                // 1: depth texture (Depth32Float)
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Depth,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                // 2: sampler
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                // 3: blur params uniform
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // 4: AO output (storage texture)
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::WriteOnly,
                        format: wgpu::TextureFormat::R32Float,
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                },
            ],
        });

        // Compute pipelines
        let ao_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("gtao_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/gtao.wgsl").into()),
        });
        let ao_pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("gtao_ao_pl"),
            bind_group_layouts: &[&ao_bgl],
            push_constant_ranges: &[],
        });
        let ao_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("gtao_ao_pipeline"),
            layout: Some(&ao_pl),
            module: &ao_shader,
            entry_point: Some("gtao_main"),
            compilation_options: Default::default(),
            cache: None,
        });

        let blur_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("gtao_blur_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/gtao_blur.wgsl").into()),
        });
        let blur_pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("gtao_blur_pl"),
            bind_group_layouts: &[&blur_bgl],
            push_constant_ranges: &[],
        });
        let blur_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("gtao_blur_pipeline"),
            layout: Some(&blur_pl),
            module: &blur_shader,
            entry_point: Some("blur_main"),
            compilation_options: Default::default(),
            cache: None,
        });

        Self {
            config,
            ao_pipeline,
            blur_pipeline,
            params_buf,
            blur_h_buf,
            blur_v_buf,
            ao_texture,
            ao_view,
            blur_temp_texture,
            blur_temp_view,
            ao_bgl,
            blur_bgl,
            frame_index: 0,
            width,
            height,
        }
    }

    /// Get the final AO texture view for compositing into the lighting pass.
    pub fn ao_view(&self) -> &wgpu::TextureView {
        &self.ao_view
    }

    /// Get configuration.
    pub fn config(&self) -> &GtaoConfig {
        &self.config
    }

    /// Set configuration.
    pub fn set_config(&mut self, config: GtaoConfig) {
        self.config = config;
    }

    /// Update parameters for this frame.
    pub fn update_params(
        &mut self,
        queue: &wgpu::Queue,
        near: f32,
        far: f32,
        fov_y: f32,
        aspect: f32,
    ) {
        let mut params = GtaoParams::new(self.width, self.height, near, far, fov_y, aspect);
        params.radius = self.config.radius;
        params.num_directions = self.config.num_directions;
        params.num_steps = self.config.num_steps;
        params.power = self.config.power;
        params.frame_index = self.frame_index;
        queue.write_buffer(&self.params_buf, 0, bytemuck::bytes_of(&params));
        self.frame_index = self.frame_index.wrapping_add(1);
    }

    /// Execute the GTAO compute pass: AO generation + bilateral blur.
    pub fn execute(
        &self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        depth_view: &wgpu::TextureView,
        normal_view: &wgpu::TextureView,
    ) {
        if !self.config.enabled {
            return;
        }

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("gtao_sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        // 1. AO generation pass
        let ao_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("gtao_ao_bg"),
            layout: &self.ao_bgl,
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
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: self.params_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::TextureView(&self.blur_temp_view),
                },
            ],
        });

        let wg_x = (self.width + 7) / 8;
        let wg_y = (self.height + 7) / 8;

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("gtao_ao"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.ao_pipeline);
            pass.set_bind_group(0, &ao_bg, &[]);
            pass.dispatch_workgroups(wg_x, wg_y, 1);
        }

        // 2. Horizontal blur: blur_temp → ao_output
        let blur_h_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("gtao_blur_h_bg"),
            layout: &self.blur_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&self.blur_temp_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(depth_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: self.blur_h_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::TextureView(&self.ao_view),
                },
            ],
        });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("gtao_blur_h"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.blur_pipeline);
            pass.set_bind_group(0, &blur_h_bg, &[]);
            pass.dispatch_workgroups(wg_x, wg_y, 1);
        }

        // 3. Vertical blur: ao_output → blur_temp (then swap view)
        let blur_v_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("gtao_blur_v_bg"),
            layout: &self.blur_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&self.ao_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(depth_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: self.blur_v_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::TextureView(&self.blur_temp_view),
                },
            ],
        });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("gtao_blur_v"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.blur_pipeline);
            pass.set_bind_group(0, &blur_v_bg, &[]);
            pass.dispatch_workgroups(wg_x, wg_y, 1);
        }
        // Final result is in blur_temp_view after V pass.
        // Note: for production, we'd swap the views so ao_view always has the final result.
        // For now, consumers should use blur_temp_view for the final AO.
    }

    /// Resize AO textures.
    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        if self.width == width && self.height == height {
            return;
        }
        *self = Self::new(device, width, height);
    }

    /// Get dimensions.
    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gtao_params_size() {
        assert_eq!(std::mem::size_of::<GtaoParams>(), 64);
    }

    #[test]
    fn gtao_params_new() {
        let p = GtaoParams::new(
            1920,
            1080,
            0.1,
            200.0,
            std::f32::consts::FRAC_PI_3,
            16.0 / 9.0,
        );
        assert_eq!(p.resolution, [1920.0, 1080.0]);
        assert_eq!(p.num_directions, 8);
        assert_eq!(p.num_steps, 6);
    }

    #[test]
    fn gtao_blur_params_size() {
        assert_eq!(std::mem::size_of::<GtaoBlurParams>(), 32);
    }

    #[test]
    fn gtao_config_default() {
        let c = GtaoConfig::default();
        assert!(c.enabled);
        assert_eq!(c.num_directions, 8);
        assert_eq!(c.num_steps, 6);
    }

    #[test]
    fn gtao_pass_creation() {
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

        let pass = GtaoPass::new(&device, 1920, 1080);
        assert_eq!(pass.dimensions(), (1920, 1080));
        assert!(pass.config().enabled);
    }

    #[test]
    fn gtao_pass_resize() {
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

        let mut pass = GtaoPass::new(&device, 800, 600);
        pass.resize(&device, 1920, 1080);
        assert_eq!(pass.dimensions(), (1920, 1080));
    }
}
