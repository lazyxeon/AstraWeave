//! Fluid Renderer
//!
//! GPU rendering pipeline for fluid particles with screen-space
//! surface reconstruction, depth smoothing, and lighting.

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    pub view_proj: [[f32; 4]; 4],
    pub inv_view_proj: [[f32; 4]; 4],
    pub view_inv: [[f32; 4]; 4],
    pub cam_pos: [f32; 4],
    pub light_dir: [f32; 4],
    pub time: f32,
    pub padding: [f32; 19],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SmoothParams {
    pub radius: i32,
    pub blur_scale: f32,
    pub blur_depth_falloff: f32,
    pub padding: [f32; 5],
}

pub struct FluidRenderer {
    depth_pipeline: wgpu::RenderPipeline,
    smooth_pipeline: wgpu::ComputePipeline,
    shade_pipeline: wgpu::RenderPipeline,
    secondary_pipeline: wgpu::RenderPipeline,

    camera_buffer: wgpu::Buffer,
    smooth_params_buffer: wgpu::Buffer,

    depth_texture: wgpu::Texture,
    smoothed_depth_texture: wgpu::Texture,

    depth_bind_group: wgpu::BindGroup,
    smooth_bind_group: wgpu::BindGroup,
    shade_bind_group_layout: wgpu::BindGroupLayout,

    width: u32,
    height: u32,
}

impl FluidRenderer {
    pub fn new(
        device: &wgpu::Device,
        width: u32,
        height: u32,
        surface_format: wgpu::TextureFormat,
    ) -> Self {
        // --- Shaders ---
        let depth_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("SSFR Depth Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/ssfr_depth.wgsl").into()),
        });
        let smooth_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("SSFR Smooth Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/ssfr_smooth_v2.wgsl").into()),
        });
        let shade_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("SSFR Shade Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/ssfr_shade.wgsl").into()),
        });

        // --- Buffers ---
        let camera_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("SSFR Camera Buffer"),
            size: std::mem::size_of::<CameraUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let params_data = &[SmoothParams {
            radius: 5,
            blur_scale: 0.1,
            blur_depth_falloff: 100.0,
            padding: [0.0; 5],
        }];
        let params_bytes: &[u8] = bytemuck::cast_slice(params_data);

        let smooth_params_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("SSFR Smooth Params"),
            size: 32,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: true,
        });
        {
            let mut slice = smooth_params_buffer.slice(..).get_mapped_range_mut();
            slice[..params_bytes.len()].copy_from_slice(params_bytes);
        }
        smooth_params_buffer.unmap();

        // --- Textures ---
        let depth_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("SSFR Raw Depth"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let smoothed_depth_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("SSFR Smoothed Depth"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R32Float,
            usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        // --- Pipelines ---
        let depth_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("SSFR Depth Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    // FRAGMENT too: ssfr_depth.wgsl's fs_main reads the
                    // camera uniform to reconstruct sphere-surface depth
                    // (view_inv/cam_pos/view_proj). VERTEX-only visibility
                    // made every FluidRenderer construction panic at
                    // pipeline creation (F.1.1 finding — the renderer had
                    // never successfully constructed on any device).
                    // secondary.wgsl shares this layout; its fragment stage
                    // ignores the uniform, so the superset is harmless.
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let depth_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("SSFR Depth Pipeline Layout"),
                bind_group_layouts: &[&depth_bind_group_layout],
                push_constant_ranges: &[],
            });

        let depth_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("SSFR Depth Pipeline"),
            layout: Some(&depth_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &depth_shader,
                entry_point: Some("vs_main"),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: 80, // Extended Particle size
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &[wgpu::VertexAttribute {
                        offset: 0,
                        shader_location: 0,
                        format: wgpu::VertexFormat::Float32x4,
                    }],
                }],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &depth_shader,
                entry_point: Some("fs_main"),
                targets: &[], // Depth only (actually we use the depth buffer)
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        // --- Smooth Compute ---
        let smooth_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("SSFR Smooth Layout"),
                entries: &[
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
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::StorageTexture {
                            access: wgpu::StorageTextureAccess::WriteOnly,
                            format: wgpu::TextureFormat::R32Float,
                            view_dimension: wgpu::TextureViewDimension::D2,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 4,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        let smooth_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("SSFR Smooth Pipeline Layout"),
                bind_group_layouts: &[&smooth_bind_group_layout],
                push_constant_ranges: &[],
            });

        let smooth_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("SSFR Smooth Pipeline"),
            layout: Some(&smooth_pipeline_layout),
            module: &smooth_shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });

        // --- Shade Rendering ---
        let shade_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("SSFR Shade Layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT | wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: false },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: false },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 4,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 5,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Depth,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 6,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
                        count: None,
                    },
                ],
            });

        let shade_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("SSFR Shade Pipeline Layout"),
                bind_group_layouts: &[&shade_bind_group_layout],
                push_constant_ranges: &[],
            });

        let shade_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("SSFR Shade Pipeline"),
            layout: Some(&shade_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shade_shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shade_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let secondary_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Secondary Particle Shader"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(include_str!(
                "../shaders/secondary.wgsl"
            ))),
        });

        let secondary_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Secondary Particle Pipeline"),
            layout: Some(&depth_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &secondary_shader,
                entry_point: Some("vs_main"),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: 48, // SecondaryParticle size
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &[
                        wgpu::VertexAttribute {
                            offset: 0,
                            shader_location: 0,
                            format: wgpu::VertexFormat::Float32x4,
                        },
                        wgpu::VertexAttribute {
                            offset: 16,
                            shader_location: 1,
                            format: wgpu::VertexFormat::Float32x4,
                        },
                        wgpu::VertexAttribute {
                            offset: 32,
                            shader_location: 2,
                            format: wgpu::VertexFormat::Float32x4,
                        },
                    ],
                }],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &secondary_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format, // Match main surface format
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::SrcAlpha,
                            dst_factor: wgpu::BlendFactor::One,
                            operation: wgpu::BlendOperation::Add,
                        },
                        alpha: wgpu::BlendComponent::REPLACE,
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: false,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        // --- Bind Groups ---
        let depth_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("SSFR Depth Bind Group"),
            layout: &depth_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });

        let smooth_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("SSFR Smooth Bind Group"),
            layout: &smooth_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(
                        &depth_texture.create_view(&Default::default()),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(
                        &smoothed_depth_texture.create_view(&Default::default()),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &smooth_params_buffer,
                        offset: 0,
                        size: std::num::NonZeroU64::new(32),
                    }),
                },
            ],
        });

        Self {
            depth_pipeline,
            smooth_pipeline,
            shade_pipeline,
            secondary_pipeline,
            camera_buffer,
            smooth_params_buffer,
            depth_texture,
            smoothed_depth_texture,
            depth_bind_group,
            smooth_bind_group,
            shade_bind_group_layout,
            width,
            height,
        }
    }

    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        self.width = width;
        self.height = height;

        self.depth_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("SSFR Raw Depth"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        self.smoothed_depth_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("SSFR Smoothed Depth"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R32Float,
            usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        // Re-create smooth bind group since it points to the old textures
        let smooth_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("SSFR Smooth Layout"),
                entries: &[
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
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::StorageTexture {
                            access: wgpu::StorageTextureAccess::WriteOnly,
                            format: wgpu::TextureFormat::R32Float,
                            view_dimension: wgpu::TextureViewDimension::D2,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 4,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        self.smooth_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("SSFR Smooth Bind Group"),
            layout: &smooth_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(
                        &self.depth_texture.create_view(&Default::default()),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(
                        &self.smoothed_depth_texture.create_view(&Default::default()),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &self.smooth_params_buffer,
                        offset: 0,
                        size: std::num::NonZeroU64::new(32),
                    }),
                },
            ],
        });
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        target_view: &wgpu::TextureView,
        scene_view: &wgpu::TextureView,
        scene_depth_view: &wgpu::TextureView,
        skybox_view: &wgpu::TextureView,
        particle_buffer: &wgpu::Buffer,
        particle_count: u32,
        secondary_particle_buffer: &wgpu::Buffer,
        secondary_particle_count: u32,
        camera: CameraUniform,
        queue: &wgpu::Queue,
        device: &wgpu::Device,
    ) {
        // Update Camera
        queue.write_buffer(&self.camera_buffer, 0, bytemuck::cast_slice(&[camera]));

        // 1. Depth Pass
        {
            let depth_view = self.depth_texture.create_view(&Default::default());
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("SSFR Depth Pass"),
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });
            rpass.set_pipeline(&self.depth_pipeline);
            rpass.set_bind_group(0, &self.depth_bind_group, &[]);
            rpass.set_vertex_buffer(0, particle_buffer.slice(..));
            rpass.draw(0..4, 0..particle_count);
        }

        // 2. Smooth Pass (Compute)
        {
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("SSFR Smooth Pass"),
                ..Default::default()
            });
            cpass.set_pipeline(&self.smooth_pipeline);
            cpass.set_bind_group(0, &self.smooth_bind_group, &[]);
            cpass.dispatch_workgroups(self.width.div_ceil(16), self.height.div_ceil(16), 1);
        }

        // 3. Shade Pass
        {
            let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some("SSFR Shade Sampler"),
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                ..Default::default()
            });

            let nearest_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some("SSFR Nearest Sampler"),
                mag_filter: wgpu::FilterMode::Nearest,
                min_filter: wgpu::FilterMode::Nearest,
                ..Default::default()
            });

            let shade_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("SSFR Shade Bind Group"),
                layout: &self.shade_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: self.camera_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(
                            &self.smoothed_depth_texture.create_view(&Default::default()),
                        ),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(scene_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::TextureView(skybox_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: wgpu::BindingResource::Sampler(&sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 5,
                        resource: wgpu::BindingResource::TextureView(scene_depth_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 6,
                        resource: wgpu::BindingResource::Sampler(&nearest_sampler),
                    },
                ],
            });

            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("SSFR Shade Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
            rpass.set_pipeline(&self.shade_pipeline);
            rpass.set_bind_group(0, &shade_bind_group, &[]);
            rpass.draw(0..3, 0..1); // Full screen triangle
        }

        // 3. Secondary Particles (Whitewater/Spray)
        if secondary_particle_count > 0 {
            let depth_view = self.depth_texture.create_view(&Default::default());
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("SSFR Secondary Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &depth_view,
                    depth_ops: None, // Only testing
                    stencil_ops: None,
                }),
                ..Default::default()
            });

            rpass.set_pipeline(&self.secondary_pipeline);
            rpass.set_bind_group(0, &self.depth_bind_group, &[]); // Layout 0 has ViewParams
            rpass.set_vertex_buffer(0, secondary_particle_buffer.slice(..));
            rpass.draw(0..4, 0..secondary_particle_count);
        }
    }

    /// F.4.2 — standalone additive-billboard accent render (the SSFR-chain split).
    ///
    /// Draws ONLY the secondary (accent) billboards — the depth → smooth → shade
    /// SSFR *surface* passes are deliberately NOT invoked here. The W-series water
    /// surface is rendered by `astraweave-render::WaterRenderer`; accents composite
    /// *over* it. This is the render half of the binary-glue seam: the binary draws
    /// the surface (render crate) then calls this to additively blend the accents
    /// into the same post-opaque HDR target. `astraweave-render` and
    /// `astraweave-fluids` stay independent — only `wgpu` types cross here.
    ///
    /// `depth_view` is the external scene depth (`Depth32Float`) the surface used,
    /// so accents are occluded by geometry/water (depth-test only, no write — the
    /// secondary pipeline has `depth_write_enabled: false`). At `secondary_count == 0`
    /// this is a no-op: no render pass is begun, so the HDR target is byte-identical
    /// (zero-accent identity).
    #[allow(clippy::too_many_arguments)]
    pub fn render_accents(
        &self,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        target_view: &wgpu::TextureView,
        depth_view: &wgpu::TextureView,
        secondary_particle_buffer: &wgpu::Buffer,
        secondary_count: u32,
        camera: CameraUniform,
    ) {
        if secondary_count == 0 {
            return; // zero-accent identity — target untouched, no pass recorded
        }
        // The billboard vertex shader reads the camera uniform (bind group 0).
        queue.write_buffer(&self.camera_buffer, 0, bytemuck::cast_slice(&[camera]));

        let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Accent Billboard Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load, // composite additively over the surface
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: depth_view,
                depth_ops: None, // test only — occlude by scene/water, no depth write
                stencil_ops: None,
            }),
            ..Default::default()
        });

        rpass.set_pipeline(&self.secondary_pipeline);
        rpass.set_bind_group(0, &self.depth_bind_group, &[]); // group 0 = ViewParams (camera)
        rpass.set_vertex_buffer(0, secondary_particle_buffer.slice(..));
        rpass.draw(0..4, 0..secondary_count);
    }
}
