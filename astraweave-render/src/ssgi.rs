//! Screen-Space Global Illumination (SSGI) with temporal reprojection.
//!
//! Provides one-bounce indirect diffuse lighting by tracing rays in screen space
//! against the depth buffer. Includes spatial bilateral filtering and temporal
//! accumulation via motion vectors for stable, noise-free results.

/// GPU-side uniform parameters for the SSGI compute pass.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SsgiParams {
    pub inv_proj: [[f32; 4]; 4],
    pub proj: [[f32; 4]; 4],
    pub resolution: [f32; 2],
    pub inv_resolution: [f32; 2],
    pub max_ray_distance: f32,
    pub ray_step_size: f32,
    pub num_rays: u32,
    pub max_steps: u32,
    pub thickness: f32,
    pub intensity: f32,
    pub frame_index: u32,
    pub _pad: u32,
}

impl Default for SsgiParams {
    fn default() -> Self {
        Self {
            inv_proj: glam::Mat4::IDENTITY.to_cols_array_2d(),
            proj: glam::Mat4::IDENTITY.to_cols_array_2d(),
            resolution: [1920.0, 1080.0],
            inv_resolution: [1.0 / 1920.0, 1.0 / 1080.0],
            max_ray_distance: 50.0,
            ray_step_size: 0.5,
            num_rays: 4,
            max_steps: 32,
            thickness: 0.5,
            intensity: 1.0,
            frame_index: 0,
            _pad: 0,
        }
    }
}

/// GPU-side uniform for SSGI temporal denoise.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SsgiDenoiseParams {
    pub inv_resolution: [f32; 2],
    pub spatial_sigma: f32,
    pub temporal_blend: f32,
    pub depth_threshold: f32,
    pub normal_threshold: f32,
    pub _pad: [f32; 2],
}

impl Default for SsgiDenoiseParams {
    fn default() -> Self {
        Self {
            inv_resolution: [1.0 / 1920.0, 1.0 / 1080.0],
            spatial_sigma: 1.5,
            temporal_blend: 0.9,
            depth_threshold: 0.05,
            normal_threshold: 0.8,
            _pad: [0.0; 2],
        }
    }
}

/// Configuration for the SSGI system.
#[derive(Debug, Clone)]
pub struct SsgiConfig {
    pub enabled: bool,
    pub num_rays: u32,
    pub max_steps: u32,
    pub ray_step_size: f32,
    pub thickness: f32,
    pub intensity: f32,
    pub temporal_blend: f32,
    /// Trace at half resolution and bilateral upsample to save bandwidth.
    pub half_res: bool,
}

impl Default for SsgiConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            num_rays: 4,
            max_steps: 32,
            ray_step_size: 0.5,
            thickness: 0.5,
            intensity: 1.0,
            temporal_blend: 0.9,
            half_res: false,
        }
    }
}

/// GPU-side uniform for SSGI bilateral upsample.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SsgiUpsampleParams {
    pub full_resolution: [f32; 2],
    pub half_resolution: [f32; 2],
    pub depth_threshold: f32,
    pub _pad: [f32; 3],
}

impl Default for SsgiUpsampleParams {
    fn default() -> Self {
        Self {
            full_resolution: [1920.0, 1080.0],
            half_resolution: [960.0, 540.0],
            depth_threshold: 0.05,
            _pad: [0.0; 3],
        }
    }
}

/// Manages SSGI GPU resources.
#[allow(dead_code)]
pub struct SsgiPass {
    config: SsgiConfig,
    /// SSGI compute pipeline.
    gi_pipeline: wgpu::ComputePipeline,
    /// Denoise compute pipeline.
    denoise_pipeline: wgpu::ComputePipeline,
    /// Bilateral upsample pipeline (used when half_res is enabled).
    upsample_pipeline: wgpu::ComputePipeline,
    /// SSGI params buffer.
    params_buf: wgpu::Buffer,
    /// Denoise params buffer.
    denoise_buf: wgpu::Buffer,
    /// Upsample params buffer.
    upsample_buf: wgpu::Buffer,
    /// Current frame GI output (rgba16float). Half-res when half_res enabled.
    #[allow(dead_code)] // texture must be kept alive for view to remain valid
    gi_texture: wgpu::Texture,
    gi_view: wgpu::TextureView,
    /// History GI texture for temporal accumulation. Same res as gi_texture.
    #[allow(dead_code)] // texture must be kept alive for view to remain valid
    history_texture: wgpu::Texture,
    history_view: wgpu::TextureView,
    /// Denoised output at trace resolution.
    #[allow(dead_code)] // texture must be kept alive for view to remain valid
    trace_denoised_texture: wgpu::Texture,
    trace_denoised_view: wgpu::TextureView,
    /// Denoised output — always full resolution.
    #[allow(dead_code)] // texture must be kept alive for view to remain valid
    denoised_texture: wgpu::Texture,
    denoised_view: wgpu::TextureView,
    /// Bind group layouts.
    gi_bgl: wgpu::BindGroupLayout,
    #[allow(dead_code)]
    denoise_bgl: wgpu::BindGroupLayout,
    upsample_bgl: wgpu::BindGroupLayout,
    frame_index: u32,
    /// Full output resolution.
    width: u32,
    height: u32,
    /// Trace resolution (half of width/height when half_res enabled).
    trace_width: u32,
    trace_height: u32,
    /// Shared linear sampler for all SSGI passes.
    sampler: wgpu::Sampler,
    /// Generation-tracked bind group cache for GI trace.
    gi_cached_bg: crate::bind_group_cache::CachedBindGroup,
    /// Generation-tracked bind group cache for temporal denoise.
    denoise_cached_bg: crate::bind_group_cache::CachedBindGroup,
    /// Generation-tracked bind group cache for bilateral upsample.
    upsample_cached_bg: crate::bind_group_cache::CachedBindGroup,
}

impl SsgiPass {
    pub fn new(device: &wgpu::Device, width: u32, height: u32) -> Self {
        Self::new_with_config(device, width, height, SsgiConfig::default())
    }

    pub fn new_with_config(
        device: &wgpu::Device,
        width: u32,
        height: u32,
        config: SsgiConfig,
    ) -> Self {
        // Trace resolution: half when half_res enabled
        let trace_width = if config.half_res { (width / 2).max(1) } else { width };
        let trace_height = if config.half_res { (height / 2).max(1) } else { height };

        let trace_size = wgpu::Extent3d {
            width: trace_width,
            height: trace_height,
            depth_or_array_layers: 1,
        };
        let full_size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };
        let gi_fmt = wgpu::TextureFormat::Rgba16Float;

        let make_tex = |label: &str, size: wgpu::Extent3d, usage: wgpu::TextureUsages| {
            device.create_texture(&wgpu::TextureDescriptor {
                label: Some(label),
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: gi_fmt,
                usage,
                view_formats: &[],
            })
        };

        // GI trace + history at trace resolution; denoised output at full resolution
        let gi_texture = make_tex(
            "ssgi_output",
            trace_size,
            wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
        );
        let gi_view = gi_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let history_texture = make_tex(
            "ssgi_history",
            trace_size,
            wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        );
        let history_view = history_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let trace_denoised_texture = make_tex(
            "ssgi_trace_denoised",
            trace_size,
            wgpu::TextureUsages::STORAGE_BINDING
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
        );
        let trace_denoised_view =
            trace_denoised_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let denoised_texture = make_tex(
            "ssgi_denoised",
            full_size,
            wgpu::TextureUsages::STORAGE_BINDING
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_DST,
        );
        let denoised_view = denoised_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("ssgi_sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        use wgpu::util::DeviceExt;
        let params = SsgiParams::default();
        let params_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("ssgi_params"),
            contents: bytemuck::bytes_of(&params),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let denoise_params = SsgiDenoiseParams::default();
        let denoise_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("ssgi_denoise_params"),
            contents: bytemuck::bytes_of(&denoise_params),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let upsample_params = SsgiUpsampleParams::default();
        let upsample_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("ssgi_upsample_params"),
            contents: bytemuck::bytes_of(&upsample_params),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // GI bind group layout
        let gi_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("ssgi_gi_bgl"),
            entries: &[
                bgl_texture(0),            // depth
                bgl_texture(1),            // normals
                bgl_texture(2),            // color
                bgl_sampler(3),            // sampler
                bgl_uniform(4),            // params
                bgl_storage_rw(5, gi_fmt), // output
            ],
        });

        // Denoise bind group layout
        let denoise_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("ssgi_denoise_bgl"),
            entries: &[
                bgl_texture(0),            // gi current
                bgl_texture(1),            // gi history
                bgl_texture(2),            // depth
                bgl_texture(3),            // velocity
                bgl_sampler(4),            // sampler
                bgl_uniform(5),            // denoise params
                bgl_storage_rw(6, gi_fmt), // output
            ],
        });

        // Bilateral upsample bind group layout (half→full res)
        let upsample_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("ssgi_upsample_bgl"),
            entries: &[
                bgl_texture(0),            // half-res GI
                bgl_texture(1),            // full-res depth
                bgl_texture(2),            // half-res depth
                bgl_sampler(3),            // sampler
                bgl_uniform(4),            // upsample params
                bgl_storage_rw(5, gi_fmt), // full-res output
            ],
        });

        // Pipelines
        let gi_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("ssgi_shader"),
            source: wgpu::ShaderSource::Wgsl(concat!(include_str!("../shaders/constants.wgsl"), include_str!("../shaders/ssgi.wgsl")).into()),
        });
        let gi_pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("ssgi_gi_pl"),
            bind_group_layouts: &[&gi_bgl],
            push_constant_ranges: &[],
        });
        let gi_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("ssgi_gi_pipeline"),
            layout: Some(&gi_pl),
            module: &gi_shader,
            entry_point: Some("ssgi_main"),
            compilation_options: Default::default(),
            cache: None,
        });

        let denoise_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("ssgi_denoise_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/ssgi_denoise.wgsl").into()),
        });
        let denoise_pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("ssgi_denoise_pl"),
            bind_group_layouts: &[&denoise_bgl],
            push_constant_ranges: &[],
        });
        let denoise_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("ssgi_denoise_pipeline"),
            layout: Some(&denoise_pl),
            module: &denoise_shader,
            entry_point: Some("denoise_main"),
            compilation_options: Default::default(),
            cache: None,
        });

        let upsample_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("ssgi_upsample_shader"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!("../shaders/ssgi_bilateral_upsample.wgsl").into(),
            ),
        });
        let upsample_pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("ssgi_upsample_pl"),
            bind_group_layouts: &[&upsample_bgl],
            push_constant_ranges: &[],
        });
        let upsample_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("ssgi_upsample_pipeline"),
            layout: Some(&upsample_pl),
            module: &upsample_shader,
            entry_point: Some("upsample_main"),
            compilation_options: Default::default(),
            cache: None,
        });

        Self {
            config,
            gi_pipeline,
            denoise_pipeline,
            upsample_pipeline,
            params_buf,
            denoise_buf,
            upsample_buf,
            gi_texture,
            gi_view,
            history_texture,
            history_view,
            trace_denoised_texture,
            trace_denoised_view,
            denoised_texture,
            denoised_view,
            gi_bgl,
            denoise_bgl,
            upsample_bgl,
            frame_index: 0,
            width,
            height,
            trace_width,
            trace_height,
            sampler,
            gi_cached_bg: crate::bind_group_cache::CachedBindGroup::new(),
            denoise_cached_bg: crate::bind_group_cache::CachedBindGroup::new(),
            upsample_cached_bg: crate::bind_group_cache::CachedBindGroup::new(),
        }
    }

    /// Get the final denoised GI texture view.
    #[allow(clippy::misnamed_getters)]
    pub fn gi_view(&self) -> &wgpu::TextureView {
        &self.denoised_view
    }

    pub fn config(&self) -> &SsgiConfig {
        &self.config
    }

    pub fn set_config(&mut self, config: SsgiConfig) {
        self.config = config;
    }

    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Update parameters for this frame.
    pub fn update_params(&mut self, queue: &wgpu::Queue, proj: glam::Mat4) {
        let params = SsgiParams {
            inv_proj: proj.inverse().to_cols_array_2d(),
            proj: proj.to_cols_array_2d(),
            resolution: [self.trace_width as f32, self.trace_height as f32],
            inv_resolution: [1.0 / self.trace_width as f32, 1.0 / self.trace_height as f32],
            max_ray_distance: 50.0,
            ray_step_size: self.config.ray_step_size,
            num_rays: self.config.num_rays,
            max_steps: self.config.max_steps,
            thickness: self.config.thickness,
            intensity: self.config.intensity,
            frame_index: self.frame_index,
            _pad: 0,
        };
        queue.write_buffer(&self.params_buf, 0, bytemuck::bytes_of(&params));

        let denoise = SsgiDenoiseParams {
            inv_resolution: [1.0 / self.trace_width as f32, 1.0 / self.trace_height as f32],
            temporal_blend: self.config.temporal_blend,
            ..SsgiDenoiseParams::default()
        };
        queue.write_buffer(&self.denoise_buf, 0, bytemuck::bytes_of(&denoise));

        if self.config.half_res {
            let upsample = SsgiUpsampleParams {
                full_resolution: [self.width as f32, self.height as f32],
                half_resolution: [self.trace_width as f32, self.trace_height as f32],
                depth_threshold: 0.05,
                _pad: [0.0; 3],
            };
            queue.write_buffer(&self.upsample_buf, 0, bytemuck::bytes_of(&upsample));
        }

        self.frame_index = self.frame_index.wrapping_add(1);
    }

    /// Dispatch SSGI trace + temporal denoise (+ bilateral upsample if half-res).
    #[allow(clippy::too_many_arguments)]
    pub fn execute(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        depth_view: &wgpu::TextureView,
        normal_view: &wgpu::TextureView,
        color_view: &wgpu::TextureView,
        velocity_view: &wgpu::TextureView,
        resource_gen: crate::bind_group_cache::Generation,
    ) {
        if !self.config.enabled {
            return;
        }

        let gi_bg = self.gi_cached_bg.get_or_rebuild(resource_gen, || {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("ssgi_gi_bg"),
                layout: &self.gi_bgl,
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
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: self.params_buf.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 5,
                        resource: wgpu::BindingResource::TextureView(&self.gi_view),
                    },
                ],
            })
        });

        let trace_wg_x = self.trace_width.div_ceil(8);
        let trace_wg_y = self.trace_height.div_ceil(8);

        {
            let mut gi_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("ssgi_gi"),
                timestamp_writes: None,
            });
            gi_pass.set_pipeline(&self.gi_pipeline);
            gi_pass.set_bind_group(0, gi_bg, &[]);
            gi_pass.dispatch_workgroups(trace_wg_x, trace_wg_y, 1);
        }

        let denoise_bg = self.denoise_cached_bg.get_or_rebuild(resource_gen, || {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("ssgi_denoise_bg"),
                layout: &self.denoise_bgl,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&self.gi_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&self.history_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(depth_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::TextureView(velocity_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 5,
                        resource: self.denoise_buf.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 6,
                        resource: wgpu::BindingResource::TextureView(&self.trace_denoised_view),
                    },
                ],
            })
        });

        {
            let mut denoise_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("ssgi_denoise"),
                timestamp_writes: None,
            });
            denoise_pass.set_pipeline(&self.denoise_pipeline);
            denoise_pass.set_bind_group(0, denoise_bg, &[]);
            denoise_pass.dispatch_workgroups(trace_wg_x, trace_wg_y, 1);
        }

        if self.config.half_res {
            let upsample_bg = self.upsample_cached_bg.get_or_rebuild(resource_gen, || {
                device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("ssgi_upsample_bg"),
                    layout: &self.upsample_bgl,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(&self.trace_denoised_view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::TextureView(depth_view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: wgpu::BindingResource::TextureView(depth_view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 3,
                            resource: wgpu::BindingResource::Sampler(&self.sampler),
                        },
                        wgpu::BindGroupEntry {
                            binding: 4,
                            resource: self.upsample_buf.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 5,
                            resource: wgpu::BindingResource::TextureView(&self.denoised_view),
                        },
                    ],
                })
            });

            let full_wg_x = self.width.div_ceil(8);
            let full_wg_y = self.height.div_ceil(8);

            {
                let mut upsample_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("ssgi_upsample"),
                    timestamp_writes: None,
                });
                upsample_pass.set_pipeline(&self.upsample_pipeline);
                upsample_pass.set_bind_group(0, upsample_bg, &[]);
                upsample_pass.dispatch_workgroups(full_wg_x, full_wg_y, 1);
            }
        } else {
            encoder.copy_texture_to_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &self.trace_denoised_texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::TexelCopyTextureInfo {
                    texture: &self.denoised_texture,
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
    }

    /// Copy current denoised trace output to history for next-frame temporal reprojection.
    pub fn copy_to_history(&self, encoder: &mut wgpu::CommandEncoder) {
        encoder.copy_texture_to_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.trace_denoised_texture,
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
                width: self.trace_width,
                height: self.trace_height,
                depth_or_array_layers: 1,
            },
        );
    }

    /// Resize all SSGI textures.
    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        if self.width == width && self.height == height {
            return;
        }
        let config = self.config.clone();
        *self = Self::new_with_config(device, width, height, config);
    }

    /// Trace resolution (may be half of output when half_res is enabled).
    pub fn trace_dimensions(&self) -> (u32, u32) {
        (self.trace_width, self.trace_height)
    }

    /// Whether this pass is configured for half-res tracing.
    pub fn is_half_res(&self) -> bool {
        self.config.half_res
    }

    /// Upsample bind group layout (exposed for callers that build bind groups).
    pub fn upsample_bgl(&self) -> &wgpu::BindGroupLayout {
        &self.upsample_bgl
    }
}

// Helper functions for bind group layout entries
fn bgl_texture(binding: u32) -> wgpu::BindGroupLayoutEntry {
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

fn bgl_sampler(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
        count: None,
    }
}

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

fn bgl_storage_rw(binding: u32, format: wgpu::TextureFormat) -> wgpu::BindGroupLayoutEntry {
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
    fn ssgi_params_size() {
        // 2 mat4x4 (128) + 4 floats (16) + 4 floats (16) + 4 u32/f32 (16) = 176
        assert_eq!(std::mem::size_of::<SsgiParams>(), 176);
    }

    #[test]
    fn ssgi_denoise_params_size() {
        assert_eq!(std::mem::size_of::<SsgiDenoiseParams>(), 32);
    }

    #[test]
    fn ssgi_upsample_params_size() {
        assert_eq!(std::mem::size_of::<SsgiUpsampleParams>(), 32);
    }

    #[test]
    fn ssgi_config_default() {
        let c = SsgiConfig::default();
        assert!(c.enabled);
        assert_eq!(c.num_rays, 4);
        assert_eq!(c.max_steps, 32);
        assert!(!c.half_res);
    }

    #[test]
    fn ssgi_pass_creation() {
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

        let pass = SsgiPass::new(&device, 1920, 1080);
        assert_eq!(pass.dimensions(), (1920, 1080));
        assert_eq!(pass.trace_dimensions(), (1920, 1080));
        assert!(!pass.is_half_res());
    }

    #[test]
    fn ssgi_pass_half_res() {
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

        let config = SsgiConfig {
            half_res: true,
            ..SsgiConfig::default()
        };
        let pass = SsgiPass::new_with_config(&device, 1920, 1080, config);
        assert_eq!(pass.dimensions(), (1920, 1080));
        assert_eq!(pass.trace_dimensions(), (960, 540));
        assert!(pass.is_half_res());
    }
}
