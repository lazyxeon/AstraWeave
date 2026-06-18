use glam::{Mat4, Vec3};
use noise::{NoiseFn, Perlin};
use wgpu::util::DeviceExt;

/// Host mirror of `ocean.wgsl::Uniforms`.
///
/// F.1.2 (H-2): the pre-fix packed layout was 144 bytes while the WGSL
/// uniform layout is 160 — `vec2<f32>` has 8-byte alignment in the uniform
/// address space, inserting a 4-byte gap after `time_scale`, and the struct
/// size rounds up to its 16-byte alignment. Every ocean draw failed
/// validation ("bound with size 144 where the shader expects 160"), i.e.
/// the ocean scenario had never rendered a single frame. The explicit pads
/// below reproduce WGSL's implicit layout.
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniforms {
    view_proj: [[f32; 4]; 4],  // offset   0
    ocean_pos: [f32; 3],       // offset  64
    time: f32,                 // offset  76
    noise_scale: f32,          // offset  80
    height_scale: f32,         // offset  84
    time_scale: f32,           // offset  88
    _pad_vec2_align: f32,      // offset  92 (vec2 8-byte alignment gap)
    wave_direction: [f32; 2],  // offset  96
    wave_direction2: [f32; 2], // offset 104
    beers_law: f32,            // offset 112
    depth_offset: f32,         // offset 116
    edge_scale: f32,           // offset 120
    metallic: f32,             // offset 124
    roughness: f32,            // offset 128
    near: f32,                 // offset 132
    far: f32,                  // offset 136
    _pad0: f32,                // offset 140
    _pad1: f32,                // offset 144
    _pad_tail: [f32; 3],       // offset 148 -> 160 (struct align round-up)
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 3],
    uv: [f32; 2],
}

impl Vertex {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
            ],
        }
    }
}

struct OceanTile {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    num_indices: u32,
    #[allow(dead_code)] // Reserved for future tile world-space positioning
    transform: Mat4,
}

pub struct OceanRenderer {
    pipeline: wgpu::RenderPipeline,
    uniform_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    tiles: Vec<OceanTile>,
    time: f32,
    camera_pos: Vec3,
}

impl OceanRenderer {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        surface_format: wgpu::TextureFormat,
    ) -> Self {
        // Generate noise textures
        let (_wave_texture, wave_view) = Self::generate_wave_texture(device, queue, 512);
        let (_wave_bump_texture, wave_bump_view) =
            Self::generate_wave_bump_texture(device, queue, 512);
        let (_normal_texture, normal_view) = Self::generate_normal_texture(device, queue, 512);
        let (_normal2_texture, normal2_view) = Self::generate_normal_texture(device, queue, 512);

        // Create sampler
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        // Create uniform buffer
        let uniforms = Uniforms {
            view_proj: Mat4::IDENTITY.to_cols_array_2d(),
            ocean_pos: [0.0, 0.0, 0.0],
            time: 0.0,
            noise_scale: 20.0,
            height_scale: 2.0,
            time_scale: 0.1,
            _pad_vec2_align: 0.0,
            wave_direction: [0.5, -0.2],
            wave_direction2: [-0.5, 0.5],
            beers_law: 2.0,
            depth_offset: 1.5,
            edge_scale: 0.3,
            metallic: 0.0,
            roughness: 0.02,
            near: 1.0,
            far: 100.0,
            _pad0: 0.0,
            _pad1: 0.0,
            _pad_tail: [0.0; 3],
        };

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Ocean Uniform Buffer"),
            contents: bytemuck::cast_slice(&[uniforms]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Create bind group layout
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Ocean Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 5,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 6,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 7,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 8,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        // Create bind group
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Ocean Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&wave_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&wave_bump_view),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: wgpu::BindingResource::TextureView(&normal_view),
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 7,
                    resource: wgpu::BindingResource::TextureView(&normal2_view),
                },
                wgpu::BindGroupEntry {
                    binding: 8,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        // Load shader
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Ocean Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("ocean.wgsl").into()),
        });

        // Create pipeline layout
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Ocean Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        // Create render pipeline
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Ocean Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Vertex::desc()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
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
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        // Generate ocean tiles (5x5 grid).
        // F.1.4 (H-2): tile_size was 50 (total extent ±125, truncated even
        // earlier by the old zfar=100). 160 gives ±400 of water; the shader's
        // horizon fog (ocean.wgsl fs_main) blends the far water into the sky
        // haze well before the mesh edge, so no hard edge is visible.
        let mut tiles = Vec::new();
        let tile_size = 160.0;
        let subdivisions = 100;

        for x in -2..=2 {
            for z in -2..=2 {
                let offset_x = x as f32 * tile_size;
                let offset_z = z as f32 * tile_size;

                // Adjust subdivision based on distance from center
                let dist = ((x * x + z * z) as f32).sqrt();
                let tile_subdivisions = if dist <= 1.0 {
                    subdivisions
                } else if dist <= 2.0 {
                    subdivisions / 2
                } else {
                    subdivisions / 4
                };

                let transform = Mat4::from_translation(Vec3::new(offset_x, 0.0, offset_z));
                let tile = Self::create_tile(device, tile_size, tile_subdivisions, transform);
                tiles.push(tile);
            }
        }

        Self {
            pipeline,
            uniform_buffer,
            bind_group,
            tiles,
            time: 0.0,
            camera_pos: Vec3::ZERO,
        }
    }

    fn create_tile(
        device: &wgpu::Device,
        size: f32,
        subdivisions: u32,
        transform: Mat4,
    ) -> OceanTile {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        let half_size = size / 2.0;
        let step = size / subdivisions as f32;

        // Generate vertices
        for z in 0..=subdivisions {
            for x in 0..=subdivisions {
                let pos_x = -half_size + x as f32 * step;
                let pos_z = -half_size + z as f32 * step;

                let world_pos = transform.transform_point3(Vec3::new(pos_x, 0.0, pos_z));

                vertices.push(Vertex {
                    position: world_pos.to_array(),
                    uv: [
                        x as f32 / subdivisions as f32,
                        z as f32 / subdivisions as f32,
                    ],
                });
            }
        }

        // Generate indices
        for z in 0..subdivisions {
            for x in 0..subdivisions {
                let top_left = z * (subdivisions + 1) + x;
                let top_right = top_left + 1;
                let bottom_left = (z + 1) * (subdivisions + 1) + x;
                let bottom_right = bottom_left + 1;

                indices.push(top_left);
                indices.push(bottom_left);
                indices.push(top_right);

                indices.push(top_right);
                indices.push(bottom_left);
                indices.push(bottom_right);
            }
        }

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Ocean Tile Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Ocean Tile Index Buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        OceanTile {
            vertex_buffer,
            index_buffer,
            num_indices: indices.len() as u32,
            transform,
        }
    }

    fn generate_wave_texture(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        size: u32,
    ) -> (wgpu::Texture, wgpu::TextureView) {
        let perlin = Perlin::new(42);
        let mut data = vec![0u8; (size * size * 4) as usize];

        for y in 0..size {
            for x in 0..size {
                let nx = x as f64 / size as f64 * 4.0;
                let ny = y as f64 / size as f64 * 4.0;

                let value = perlin.get([nx, ny]);
                let normalized = ((value + 1.0) / 2.0 * 255.0) as u8;

                let idx = ((y * size + x) * 4) as usize;
                data[idx] = normalized;
                data[idx + 1] = normalized;
                data[idx + 2] = normalized;
                data[idx + 3] = 255;
            }
        }

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Wave Texture"),
            size: wgpu::Extent3d {
                width: size,
                height: size,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * size),
                rows_per_image: Some(size),
            },
            wgpu::Extent3d {
                width: size,
                height: size,
                depth_or_array_layers: 1,
            },
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        (texture, view)
    }

    fn generate_wave_bump_texture(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        size: u32,
    ) -> (wgpu::Texture, wgpu::TextureView) {
        // Similar to wave texture but with different noise settings
        Self::generate_wave_texture(device, queue, size)
    }

    fn generate_normal_texture(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        size: u32,
    ) -> (wgpu::Texture, wgpu::TextureView) {
        // F.1.3 (H-2): pre-fix this wrote a DEGENERATE "normal map" — constant
        // (128, 128, noise): zero x/y tilt with a noisy z, which after the
        // shader's *2-1 decode normalizes to a flat (or sign-flipped) normal.
        // The entire normal-perturbation path was inert — one ingredient of
        // the owner's "gelatin" (no surface detail response to view/light).
        // Now: a proper tangent-space normal map derived from the height
        // field's finite differences.
        let perlin = Perlin::new(123);
        let mut data = vec![0u8; (size * size * 4) as usize];

        let height = |x: i64, y: i64| -> f64 {
            // Wrap for seamless tiling.
            let xw = x.rem_euclid(size as i64) as f64;
            let yw = y.rem_euclid(size as i64) as f64;
            perlin.get([xw / size as f64 * 8.0, yw / size as f64 * 8.0])
        };
        let strength = 2.0_f64;

        for y in 0..size as i64 {
            for x in 0..size as i64 {
                let dx = height(x - 1, y) - height(x + 1, y);
                let dy = height(x, y - 1) - height(x, y + 1);
                let len = (dx * dx + dy * dy + strength * strength).sqrt();
                let n = [dx / len, dy / len, strength / len];

                let idx = ((y as u32 * size + x as u32) * 4) as usize;
                data[idx] = ((n[0] * 0.5 + 0.5) * 255.0) as u8;
                data[idx + 1] = ((n[1] * 0.5 + 0.5) * 255.0) as u8;
                data[idx + 2] = ((n[2] * 0.5 + 0.5) * 255.0) as u8;
                data[idx + 3] = 255;
            }
        }

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Normal Texture"),
            size: wgpu::Extent3d {
                width: size,
                height: size,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * size),
                rows_per_image: Some(size),
            },
            wgpu::Extent3d {
                width: size,
                height: size,
                depth_or_array_layers: 1,
            },
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        (texture, view)
    }

    pub fn update(&mut self, dt: f32, camera_pos: Vec3) {
        self.time += dt;
        self.camera_pos = camera_pos;
    }

    pub fn render(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        depth_view: &wgpu::TextureView,
        queue: &wgpu::Queue,
        view_proj: Mat4,
    ) {
        // Update uniforms
        let uniforms = Uniforms {
            view_proj: view_proj.to_cols_array_2d(),
            ocean_pos: self.camera_pos.to_array(),
            time: self.time,
            noise_scale: 20.0,
            height_scale: 2.0,
            time_scale: 0.1,
            _pad_vec2_align: 0.0,
            wave_direction: [0.5, -0.2],
            wave_direction2: [-0.5, 0.5],
            beers_law: 2.0,
            depth_offset: 1.5,
            edge_scale: 0.3,
            metallic: 0.0,
            roughness: 0.02,
            near: 1.0,
            far: 100.0,
            _pad0: 0.0,
            _pad1: 0.0,
            _pad_tail: [0.0; 3],
        };

        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Ocean Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: depth_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.bind_group, &[]);

        // Render all tiles
        for tile in &self.tiles {
            render_pass.set_vertex_buffer(0, tile.vertex_buffer.slice(..));
            render_pass.set_index_buffer(tile.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            render_pass.draw_indexed(0..tile.num_indices, 0, 0..1);
        }
    }
}
