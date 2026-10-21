//! Bevy Shadow Demo - Visual Validation of CSM Implementation
//!
//! This demo creates a simple scene to validate shadow rendering:
//! - Ground plane (receives shadows)
//! - Cube (casts shadows)
//! - Directional light (sun with 4-cascade CSM)
//!
//! ## Controls
//! - ESC: Exit
//! - Arrow keys: Rotate camera
//! - W/S: Move camera forward/backward
//! - A/D: Strafe left/right
//! - Q/E: Move camera down/up

use std::sync::Arc;

use anyhow::Result;
use glam::{Mat4, Quat, Vec3};
use wgpu::util::DeviceExt;
use winit::{
    application::ApplicationHandler,
    event::{ElementState, KeyEvent, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId},
};

use astraweave_ecs::World;
use astraweave_render_bevy::{
    CascadeShadowConfig, DirectionalLight, RenderAdapter, RenderMaterial, RenderMesh,
    RenderTransform, ShadowRenderer, CASCADE_COUNT, CASCADE_RESOLUTION,
};

struct DemoApp {
    window: Option<Arc<Window>>,
    device: Option<wgpu::Device>,
    queue: Option<wgpu::Queue>,
    surface: Option<wgpu::Surface<'static>>,
    surface_config: Option<wgpu::SurfaceConfiguration>,

    // ECS world
    world: World,
    adapter: RenderAdapter,
    shadow_renderer: Option<ShadowRenderer>,

    // Render pipelines
    shadow_pipeline: Option<wgpu::RenderPipeline>,
    main_pipeline: Option<wgpu::RenderPipeline>,
    depth_texture: Option<wgpu::Texture>,
    depth_view: Option<wgpu::TextureView>,
    // Separate uniform buffers to avoid overwrite bug (wgpu queue commands are deferred!)
    ground_uniform_buffer: Option<wgpu::Buffer>,
    cube_uniform_buffer: Option<wgpu::Buffer>,
    // Dedicated shadow uniform buffers (separate from main buffers)
    shadow_ground_uniform_buffer: Option<wgpu::Buffer>,
    shadow_cube_uniform_buffer: Option<wgpu::Buffer>,
    shadow_ground_bind_group: Option<wgpu::BindGroup>,
    shadow_cube_bind_group: Option<wgpu::BindGroup>,
    ground_bind_group: Option<wgpu::BindGroup>,
    cube_bind_group: Option<wgpu::BindGroup>,
    shadow_bind_group: Option<wgpu::BindGroup>, // Group 1: shadow texture, sampler, cascades
    vertex_buffer: Option<wgpu::Buffer>,
    index_buffer: Option<wgpu::Buffer>,

    // Camera state
    camera_position: Vec3,
    camera_yaw: f32,
    camera_pitch: f32,

    // Animation state
    rotation_angle: f32,
}

impl Default for DemoApp {
    fn default() -> Self {
        Self {
            window: None,
            device: None,
            queue: None,
            surface: None,
            surface_config: None,
            world: World::new(),
            adapter: RenderAdapter::new(),
            shadow_renderer: None,
            shadow_pipeline: None,
            main_pipeline: None,
            depth_texture: None,
            depth_view: None,
            ground_uniform_buffer: None,
            cube_uniform_buffer: None,
            shadow_ground_uniform_buffer: None,
            shadow_cube_uniform_buffer: None,
            shadow_ground_bind_group: None,
            shadow_cube_bind_group: None,
            ground_bind_group: None,
            cube_bind_group: None,
            shadow_bind_group: None,
            vertex_buffer: None,
            index_buffer: None,
            camera_position: Vec3::new(3.0, 3.0, 8.0), // Closer camera for better shadow visibility
            camera_yaw: 0.0,
            camera_pitch: -30.0_f32.to_radians(),
            rotation_angle: 0.0,
        }
    }
}

impl DemoApp {
    fn setup_scene(&mut self) {
        // Ground plane (receives shadows)
        let ground = self.world.spawn();
        self.world.insert(
            ground,
            RenderTransform {
                translation: Vec3::new(0.0, 0.0, 0.0),
                rotation: Quat::IDENTITY,
                scale: Vec3::new(10.0, 0.1, 10.0),
            },
        );
        self.world.insert(ground, RenderMesh { handle: 0 }); // Cube mesh (placeholder)
        self.world.insert(
            ground,
            RenderMaterial {
                base_color: [0.3, 0.6, 0.3, 1.0], // Green
                base_color_texture: None,
                normal_texture: None,
                metallic_roughness_texture: None,
                metallic: 0.0,
                roughness: 0.8,
            },
        );

        // Cube (casts shadow)
        let cube = self.world.spawn();
        self.world.insert(
            cube,
            RenderTransform {
                translation: Vec3::new(0.0, 1.5, 0.0),
                rotation: Quat::from_rotation_y(0.785), // 45 degrees
                scale: Vec3::splat(1.0),
            },
        );
        self.world.insert(cube, RenderMesh { handle: 0 });
        self.world.insert(
            cube,
            RenderMaterial {
                base_color: [0.8, 0.2, 0.2, 1.0], // Red
                base_color_texture: None,
                normal_texture: None,
                metallic_roughness_texture: None,
                metallic: 0.1,
                roughness: 0.5,
            },
        );

        // Directional light (sun)
        let light = self.world.spawn();
        self.world.insert(
            light,
            DirectionalLight {
                direction: Vec3::new(-0.3, -1.0, -0.5).normalize(),
                color: Vec3::new(1.0, 0.95, 0.9), // Warm sunlight
                illuminance: 100_000.0,           // 100k lux (bright sunlight)
                shadows_enabled: true,
            },
        );

        println!("✅ Scene setup complete:");
        println!("   - Ground plane (green, receives shadows)");
        println!("   - Cube (red, casts shadow)");
        println!("   - Directional light (sun, 100k lux)");
    }

    /// Create render pipelines and resources
    fn create_render_resources(&mut self) {
        let device = self.device.as_ref().unwrap();
        let config = self.surface_config.as_ref().unwrap();

        // Create depth texture
        let depth_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Depth Texture"),
            size: wgpu::Extent3d {
                width: config.width,
                height: config.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let depth_view = depth_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Create cube geometry (simple unit cube)
        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct Vertex {
            position: [f32; 3],
            normal: [f32; 3],
        }

        let vertices: &[Vertex] = &[
            // Cube faces (for the floating cube)
            // Front face
            Vertex {
                position: [-0.5, -0.5, 0.5],
                normal: [0.0, 0.0, 1.0],
            },
            Vertex {
                position: [0.5, -0.5, 0.5],
                normal: [0.0, 0.0, 1.0],
            },
            Vertex {
                position: [0.5, 0.5, 0.5],
                normal: [0.0, 0.0, 1.0],
            },
            Vertex {
                position: [-0.5, 0.5, 0.5],
                normal: [0.0, 0.0, 1.0],
            },
            // Back face
            Vertex {
                position: [0.5, -0.5, -0.5],
                normal: [0.0, 0.0, -1.0],
            },
            Vertex {
                position: [-0.5, -0.5, -0.5],
                normal: [0.0, 0.0, -1.0],
            },
            Vertex {
                position: [-0.5, 0.5, -0.5],
                normal: [0.0, 0.0, -1.0],
            },
            Vertex {
                position: [0.5, 0.5, -0.5],
                normal: [0.0, 0.0, -1.0],
            },
            // Top face
            Vertex {
                position: [-0.5, 0.5, 0.5],
                normal: [0.0, 1.0, 0.0],
            },
            Vertex {
                position: [0.5, 0.5, 0.5],
                normal: [0.0, 1.0, 0.0],
            },
            Vertex {
                position: [0.5, 0.5, -0.5],
                normal: [0.0, 1.0, 0.0],
            },
            Vertex {
                position: [-0.5, 0.5, -0.5],
                normal: [0.0, 1.0, 0.0],
            },
            // Bottom face
            Vertex {
                position: [-0.5, -0.5, -0.5],
                normal: [0.0, -1.0, 0.0],
            },
            Vertex {
                position: [0.5, -0.5, -0.5],
                normal: [0.0, -1.0, 0.0],
            },
            Vertex {
                position: [0.5, -0.5, 0.5],
                normal: [0.0, -1.0, 0.0],
            },
            Vertex {
                position: [-0.5, -0.5, 0.5],
                normal: [0.0, -1.0, 0.0],
            },
            // Right face
            Vertex {
                position: [0.5, -0.5, 0.5],
                normal: [1.0, 0.0, 0.0],
            },
            Vertex {
                position: [0.5, -0.5, -0.5],
                normal: [1.0, 0.0, 0.0],
            },
            Vertex {
                position: [0.5, 0.5, -0.5],
                normal: [1.0, 0.0, 0.0],
            },
            Vertex {
                position: [0.5, 0.5, 0.5],
                normal: [1.0, 0.0, 0.0],
            },
            // Left face
            Vertex {
                position: [-0.5, -0.5, -0.5],
                normal: [-1.0, 0.0, 0.0],
            },
            Vertex {
                position: [-0.5, -0.5, 0.5],
                normal: [-1.0, 0.0, 0.0],
            },
            Vertex {
                position: [-0.5, 0.5, 0.5],
                normal: [-1.0, 0.0, 0.0],
            },
            Vertex {
                position: [-0.5, 0.5, -0.5],
                normal: [-1.0, 0.0, 0.0],
            },
            // Ground plane (large quad at y=0)
            Vertex {
                position: [-15.0, 0.0, -15.0],
                normal: [0.0, 1.0, 0.0],
            },
            Vertex {
                position: [15.0, 0.0, -15.0],
                normal: [0.0, 1.0, 0.0],
            },
            Vertex {
                position: [15.0, 0.0, 15.0],
                normal: [0.0, 1.0, 0.0],
            },
            Vertex {
                position: [-15.0, 0.0, 15.0],
                normal: [0.0, 1.0, 0.0],
            },
        ];

        let indices: &[u16] = &[
            // Cube indices
            0, 1, 2, 0, 2, 3, // Front
            4, 5, 6, 4, 6, 7, // Back
            8, 9, 10, 8, 10, 11, // Top
            12, 13, 14, 12, 14, 15, // Bottom
            16, 17, 18, 16, 18, 19, // Right
            20, 21, 22, 20, 22, 23, // Left
            // Ground plane indices (starting at vertex 24)
            24, 25, 26, 24, 26, 27,
        ];

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Cube Vertex Buffer"),
            contents: bytemuck::cast_slice(vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Cube Index Buffer"),
            contents: bytemuck::cast_slice(indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        // Create separate uniform buffers for ground and cube to avoid wgpu queue overwrite bug
        // (queue.write_buffer commands are deferred - second write would overwrite first!)
        let ground_uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Ground Uniform Buffer"),
            size: 512,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let cube_uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Cube Uniform Buffer"),
            size: 512,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Create SEPARATE uniform buffers for shadow rendering to avoid overwriting main buffers
        let shadow_ground_uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Shadow Ground Uniform Buffer"),
            size: 256, // ShadowUniforms: 2 × mat4x4 = 128 bytes (padded to 256)
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let shadow_cube_uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Shadow Cube Uniform Buffer"),
            size: 256,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Create bind group layout (shared by both ground and cube)
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Bind Group Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let ground_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Ground Bind Group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: ground_uniform_buffer.as_entire_binding(),
            }],
        });

        let cube_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Cube Bind Group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: cube_uniform_buffer.as_entire_binding(),
            }],
        });

        // Create shadow bind groups (for shadow depth pass)
        let shadow_ground_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Shadow Ground Bind Group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: shadow_ground_uniform_buffer.as_entire_binding(),
            }],
        });

        let shadow_cube_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Shadow Cube Bind Group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: shadow_cube_uniform_buffer.as_entire_binding(),
            }],
        });

        // Initialize shadow renderer (CSM with 4 cascades)
        let shadow_renderer = ShadowRenderer::new(&device, CascadeShadowConfig::default());

        // Create shadow bind group layout (group 1 in shader)
        let shadow_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Shadow Bind Group Layout"),
                entries: &[
                    // Binding 0: Shadow texture array
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Depth,
                            view_dimension: wgpu::TextureViewDimension::D2Array,
                            multisampled: false,
                        },
                        count: None,
                    },
                    // Binding 1: Shadow sampler (comparison)
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                        count: None,
                    },
                    // Binding 2: Cascade data buffer
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        // Create shadow bind group
        let shadow_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Shadow Bind Group"),
            layout: &shadow_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&shadow_renderer.shadow_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&shadow_renderer.shadow_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: shadow_renderer.cascade_buffer.as_entire_binding(),
                },
            ],
        });

        println!("✨ Shadow renderer initialized:");
        println!("   Cascades: {}", CASCADE_COUNT);
        println!(
            "   Resolution: {}×{} per cascade",
            CASCADE_RESOLUTION, CASCADE_RESOLUTION
        );
        println!(
            "   Total shadow map memory: ~{} MB",
            (CASCADE_COUNT * CASCADE_RESOLUTION as usize * CASCADE_RESOLUTION as usize * 4)
                / (1024 * 1024)
        );

        // Simple shaders (minimal for proof of concept)
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Main Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders.wgsl").into()),
        });

        // Create main pipeline layout with BOTH bind group layouts
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout, &shadow_bind_group_layout], // Group 0 + Group 1
            push_constant_ranges: &[],
        });

        let main_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Main Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3],
                }],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None, // Disable backface culling to ensure ground is visible
                ..Default::default()
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

        // Create shadow depth rendering pipeline (uses simpler layout - only group 0)
        let shadow_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Shadow Pipeline Layout"),
                bind_group_layouts: &[&bind_group_layout], // Only group 0 (uniforms), no shadow resources
                push_constant_ranges: &[],
            });

        let shadow_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Shadow Depth Pipeline"),
            layout: Some(&shadow_pipeline_layout), // Separate simpler layout
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("shadow_vs_main"),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    // Shadow shader only needs position (location 0)
                    attributes: &wgpu::vertex_attr_array![0 => Float32x3],
                }],
                compilation_options: Default::default(),
            },
            fragment: None, // Depth-only rendering, no fragment shader needed
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back), // Cull back faces for shadows
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState {
                    constant: 2, // Depth bias to prevent shadow acne
                    slope_scale: 2.0,
                    clamp: 0.0,
                },
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        self.depth_texture = Some(depth_texture);
        self.depth_view = Some(depth_view);
        self.vertex_buffer = Some(vertex_buffer);
        self.index_buffer = Some(index_buffer);
        self.ground_uniform_buffer = Some(ground_uniform_buffer);
        self.cube_uniform_buffer = Some(cube_uniform_buffer);
        self.shadow_ground_uniform_buffer = Some(shadow_ground_uniform_buffer);
        self.shadow_cube_uniform_buffer = Some(shadow_cube_uniform_buffer);
        self.ground_bind_group = Some(ground_bind_group);
        self.cube_bind_group = Some(cube_bind_group);
        self.shadow_ground_bind_group = Some(shadow_ground_bind_group);
        self.shadow_cube_bind_group = Some(shadow_cube_bind_group);
        self.shadow_bind_group = Some(shadow_bind_group);
        self.main_pipeline = Some(main_pipeline);
        self.shadow_pipeline = Some(shadow_pipeline);
        self.shadow_renderer = Some(shadow_renderer);
    }

    fn camera_rotation(&self) -> Quat {
        Quat::from_euler(glam::EulerRot::YXZ, self.camera_yaw, self.camera_pitch, 0.0)
    }

    fn camera_forward_vec(&self) -> Vec3 {
        self.camera_rotation() * Vec3::NEG_Z
    }

    fn camera_right_vec(&self) -> Vec3 {
        self.camera_rotation() * Vec3::X
    }

    fn camera_up_vec(&self) -> Vec3 {
        self.camera_rotation() * Vec3::Y
    }

    fn translate_camera(&mut self, delta: Vec3) {
        self.camera_position += delta;
    }

    fn camera_view_matrix(&self) -> Mat4 {
        let forward = self.camera_forward_vec();
        let target = self.camera_position + forward;
        Mat4::look_at_rh(self.camera_position, target, Vec3::Y)
    }

    fn camera_projection_matrix(&self, aspect_ratio: f32) -> Mat4 {
        Mat4::perspective_rh(60.0_f32.to_radians(), aspect_ratio, 0.1, 100.0)
    }

    fn update_and_render(&mut self) -> Result<()> {
        let device = self.device.as_ref().unwrap();
        let queue = self.queue.as_ref().unwrap();
        let surface = self.surface.as_ref().unwrap();
        let config = self.surface_config.as_ref().unwrap();

        // Extract render data from ECS
        self.adapter.extract_all(&self.world)?;

        // Calculate shadow cascades
        let aspect_ratio = config.width as f32 / config.height as f32;
        let view_mat = self.camera_view_matrix();
        let proj_mat = self.camera_projection_matrix(aspect_ratio);

        if let Some(shadow_renderer) = &mut self.shadow_renderer {
            // Get directional light direction
            let lights = self.adapter.directional_lights();
            if let Some(light_data) = lights.first() {
                shadow_renderer.calculate_cascades(
                    &view_mat,
                    &proj_mat,
                    light_data.light.direction.normalize(),
                );

                static mut PRINTED_CASCADE_DEBUG: bool = false;
                unsafe {
                    if !PRINTED_CASCADE_DEBUG {
                        let cube_center = Vec3::new(0.0, 1.5, 0.0);
                        let view_pos = view_mat * cube_center.extend(1.0);
                        let view_depth = (-view_pos.z).max(0.0);
                        let mut cascade_idx = 0usize;
                        for (i, cascade) in shadow_renderer.cascades.iter().enumerate() {
                            if view_depth > cascade.far {
                                cascade_idx = i + 1;
                            }
                        }
                        cascade_idx = cascade_idx.min(CASCADE_COUNT - 1);
                        println!(
                            "🎯 Cube center depth {:.3}m uses cascade {} (near {:.3}, far {:.3})",
                            view_depth,
                            cascade_idx,
                            shadow_renderer.cascades[cascade_idx].near,
                            shadow_renderer.cascades[cascade_idx].far
                        );
                        let light_space = shadow_renderer.cascades[cascade_idx].view_proj_matrix
                            * cube_center.extend(1.0);
                        let ndc = light_space.truncate() / light_space.w;
                        println!(
                            "   -> Light clip {:?}, uv {:?}",
                            light_space,
                            Vec3::new(ndc.x * 0.5 + 0.5, ndc.y * 0.5 + 0.5, ndc.z * 0.5 + 0.5)
                        );
                        let ground_point = Vec3::new(0.0, 0.0, 0.0);
                        let ground_light = shadow_renderer.cascades[cascade_idx].view_proj_matrix
                            * ground_point.extend(1.0);
                        let ground_ndc = ground_light.truncate() / ground_light.w;
                        println!(
                            "   -> Ground shadow candidate clip {:?}, uv {:?}",
                            ground_light,
                            Vec3::new(
                                ground_ndc.x * 0.5 + 0.5,
                                ground_ndc.y * 0.5 + 0.5,
                                ground_ndc.z * 0.5 + 0.5
                            )
                        );
                        PRINTED_CASCADE_DEBUG = true;
                    }
                }

                shadow_renderer.update_uniforms(queue);

                // Print cascade info (first frame only)
                static mut PRINTED: bool = false;
                unsafe {
                    if !PRINTED {
                        println!("\n🌞 Shadow Cascade Info:");
                        for (i, cascade) in shadow_renderer.cascades.iter().enumerate() {
                            println!(
                                "   Cascade {}: {:.1}m → {:.1}m",
                                i, cascade.near, cascade.far
                            );
                        }
                        PRINTED = true;
                    }
                }
            }
        }

        // Get next frame
        let frame = surface.get_current_texture()?;
        let frame_view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        let view_proj = proj_mat * view_mat;

        // === SHADOW RENDERING PASS ===
        // Render scene from light's perspective into shadow maps (4 cascades)
        if let (Some(shadow_renderer), Some(shadow_pipeline), Some(vbuf), Some(ibuf)) = (
            self.shadow_renderer.as_mut(),
            &self.shadow_pipeline,
            &self.vertex_buffer,
            &self.index_buffer,
        ) {
            // DEBUG: Print first cascade matrix (first frame only)
            static mut PRINTED_CASCADE: bool = false;
            unsafe {
                if !PRINTED_CASCADE {
                    let c0 = &shadow_renderer.cascades[0];
                    println!("\n🔍 DEBUG: Cascade 0 view_proj matrix:");
                    println!("   {:?}", c0.view_proj_matrix);
                    println!("   Near: {}, Far: {}", c0.near, c0.far);
                    PRINTED_CASCADE = true;
                }
            }

            // Update cascade uniform buffer with latest cascade data
            // MUST match shader CascadeData struct: mat4x4<f32> + vec4<f32> = 80 bytes each
            #[repr(C)]
            #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
            struct CascadeUniform {
                view_proj: [[f32; 4]; 4], // 64 bytes
                split_distances: [f32; 4], // 16 bytes
                                          // Total: 80 bytes × 4 cascades = 320 bytes
            }

            let cascade_data: Vec<CascadeUniform> = shadow_renderer
                .cascades
                .iter()
                .map(|c| CascadeUniform {
                    view_proj: c.view_proj_matrix.to_cols_array_2d(),
                    split_distances: [c.near, c.far, 0.0, 0.0],
                })
                .collect();

            queue.write_buffer(
                &shadow_renderer.cascade_buffer,
                0,
                bytemuck::cast_slice(&cascade_data),
            );

            // Render each cascade
            for (cascade_idx, cascade) in shadow_renderer.cascades.iter().enumerate() {
                // Create view for this cascade layer
                let cascade_view =
                    shadow_renderer
                        .shadow_texture
                        .create_view(&wgpu::TextureViewDescriptor {
                            label: Some(&format!("Cascade {} View", cascade_idx)),
                            format: Some(wgpu::TextureFormat::Depth32Float),
                            dimension: Some(wgpu::TextureViewDimension::D2),
                            aspect: wgpu::TextureAspect::DepthOnly,
                            base_mip_level: 0,
                            mip_level_count: Some(1),
                            base_array_layer: cascade_idx as u32,
                            array_layer_count: Some(1),
                            usage: None,
                        });

                let mut shadow_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some(&format!("Shadow Pass Cascade {}", cascade_idx)),
                    color_attachments: &[],
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                        view: &cascade_view,
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Clear(1.0),
                            store: wgpu::StoreOp::Store,
                        }),
                        stencil_ops: None,
                    }),
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });

                shadow_pass.set_pipeline(shadow_pipeline);
                shadow_pass.set_vertex_buffer(0, vbuf.slice(..));
                shadow_pass.set_index_buffer(ibuf.slice(..), wgpu::IndexFormat::Uint16);

                // Render ground into shadow map
                {
                    #[repr(C)]
                    #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
                    struct ShadowUniform {
                        light_view_proj: [[f32; 4]; 4],
                        model: [[f32; 4]; 4],
                    }

                    let shadow_uniform = ShadowUniform {
                        light_view_proj: cascade.view_proj_matrix.to_cols_array_2d(),
                        model: Mat4::IDENTITY.to_cols_array_2d(), // Ground at world origin
                    };

                    queue.write_buffer(
                        self.shadow_ground_uniform_buffer.as_ref().unwrap(), // Use SHADOW buffer, not main!
                        0,
                        bytemuck::bytes_of(&shadow_uniform),
                    );

                    shadow_pass.set_bind_group(
                        0,
                        self.shadow_ground_bind_group.as_ref().unwrap(),
                        &[],
                    );
                    shadow_pass.draw_indexed(36..42, 0, 0..1); // Ground indices
                }

                // Render cube into shadow map
                {
                    #[repr(C)]
                    #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
                    struct ShadowUniform {
                        light_view_proj: [[f32; 4]; 4],
                        model: [[f32; 4]; 4],
                    }

                    let cube_model = Mat4::from_scale_rotation_translation(
                        Vec3::ONE,
                        Quat::from_rotation_y(self.rotation_angle),
                        Vec3::new(0.0, 1.5, 0.0),
                    );

                    let shadow_uniform = ShadowUniform {
                        light_view_proj: cascade.view_proj_matrix.to_cols_array_2d(),
                        model: cube_model.to_cols_array_2d(),
                    };

                    queue.write_buffer(
                        self.shadow_cube_uniform_buffer.as_ref().unwrap(), // Use SHADOW buffer, not main!
                        0,
                        bytemuck::bytes_of(&shadow_uniform),
                    );

                    shadow_pass.set_bind_group(
                        0,
                        self.shadow_cube_bind_group.as_ref().unwrap(),
                        &[],
                    );
                    shadow_pass.draw_indexed(0..36, 0, 0..1); // Cube indices
                }
            }

            // Debug output (first frame only)
            static mut SHADOW_PRINTED: bool = false;
            unsafe {
                if !SHADOW_PRINTED {
                    println!("🌑 Shadow maps rendered: {} cascades", CASCADE_COUNT);
                    SHADOW_PRINTED = true;
                }
            }
        }

        // === MAIN RENDERING PASS ===
        // Render ground and cube
        if let (
            Some(pipeline),
            Some(ground_bg),
            Some(cube_bg),
            Some(shadow_bg),
            Some(vbuf),
            Some(ibuf),
            Some(depth_view),
        ) = (
            &self.main_pipeline,
            &self.ground_bind_group,
            &self.cube_bind_group,
            &self.shadow_bind_group,
            &self.vertex_buffer,
            &self.index_buffer,
            &self.depth_view,
        ) {
            // Get directional light direction
            let light_dir = self
                .adapter
                .directional_lights()
                .first()
                .map(|l| l.light.direction)
                .unwrap_or(Vec3::new(-0.3, -1.0, -0.5));
            let view_matrix = view_mat.to_cols_array_2d();

            // Render ground plane
            {
                // Ground stays STATIC at y=0, no rotation, no animation
                let model = Mat4::IDENTITY; // Ground vertices are already at correct world positions

                // Debug output (first frame only)
                static mut PRINTED_GROUND: bool = false;
                unsafe {
                    if !PRINTED_GROUND {
                        println!("🟩 Rendering STATIC ground plane (30×30 quad at y=0)");
                        PRINTED_GROUND = true;
                    }
                }

                #[repr(C)]
                #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
                struct UniformData {
                    view_proj: [[f32; 4]; 4],
                    view: [[f32; 4]; 4],
                    model: [[f32; 4]; 4],
                    light_dir: [f32; 4],
                    _padding: [f32; 12], // Pad to 256 bytes
                }

                let uniform_data = UniformData {
                    view_proj: view_proj.to_cols_array_2d(),
                    view: view_matrix,
                    model: model.to_cols_array_2d(),
                    light_dir: [light_dir.x, light_dir.y, light_dir.z, 0.0],
                    _padding: [0.0; 12],
                };

                // Write to GROUND uniform buffer (no overwrite!)
                queue.write_buffer(
                    self.ground_uniform_buffer.as_ref().unwrap(),
                    0,
                    bytemuck::bytes_of(&uniform_data),
                );

                let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Ground Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &frame_view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                r: 0.53,
                                g: 0.81,
                                b: 0.92,
                                a: 1.0,
                            }),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                        view: depth_view,
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Clear(1.0),
                            store: wgpu::StoreOp::Store,
                        }),
                        stencil_ops: None,
                    }),
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });

                render_pass.set_pipeline(pipeline);
                render_pass.set_bind_group(0, ground_bg, &[]); // Per-object uniforms
                render_pass.set_bind_group(1, shadow_bg, &[]); // Shadow resources (texture, sampler, cascades)
                render_pass.set_vertex_buffer(0, vbuf.slice(..));
                render_pass.set_index_buffer(ibuf.slice(..), wgpu::IndexFormat::Uint16);
                render_pass.draw_indexed(36..42, 0, 0..1); // Ground plane (indices 36-41)
            }

            // Render cube
            {
                // Animate rotation
                self.rotation_angle += 0.01; // Rotate ~0.6 degrees per frame

                let model = Mat4::from_scale_rotation_translation(
                    Vec3::ONE,
                    Quat::from_rotation_y(self.rotation_angle),
                    Vec3::new(0.0, 1.5, 0.0),
                );

                // Debug output (first frame only)
                static mut PRINTED_CUBE: bool = false;
                unsafe {
                    if !PRINTED_CUBE {
                        println!("🟥 Rendering cube: pos=(0, 1.5, 0), animated rotation");
                        PRINTED_CUBE = true;
                    }
                }

                #[repr(C)]
                #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
                struct UniformData {
                    view_proj: [[f32; 4]; 4],
                    view: [[f32; 4]; 4],
                    model: [[f32; 4]; 4],
                    light_dir: [f32; 4],
                    _padding: [f32; 12],
                }

                let uniform_data = UniformData {
                    view_proj: view_proj.to_cols_array_2d(),
                    view: view_matrix,
                    model: model.to_cols_array_2d(),
                    light_dir: [light_dir.x, light_dir.y, light_dir.z, 0.0],
                    _padding: [0.0; 12],
                };

                // Write to CUBE uniform buffer (separate from ground!)
                queue.write_buffer(
                    self.cube_uniform_buffer.as_ref().unwrap(),
                    0,
                    bytemuck::bytes_of(&uniform_data),
                );

                let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Cube Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &frame_view,
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

                render_pass.set_pipeline(pipeline);
                render_pass.set_bind_group(0, cube_bg, &[]); // Per-object uniforms
                render_pass.set_bind_group(1, shadow_bg, &[]); // Shadow resources (texture, sampler, cascades)
                render_pass.set_vertex_buffer(0, vbuf.slice(..));
                render_pass.set_index_buffer(ibuf.slice(..), wgpu::IndexFormat::Uint16);
                render_pass.draw_indexed(0..36, 0, 0..1); // Cube (indices 0-35)
            }
        }

        queue.submit(Some(encoder.finish()));
        frame.present();

        Ok(())
    }
}

impl ApplicationHandler for DemoApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        // Create window
        let window_attrs = Window::default_attributes()
            .with_title("Bevy Shadow Demo - CSM Validation")
            .with_inner_size(winit::dpi::PhysicalSize::new(1280, 720));

        let window = Arc::new(event_loop.create_window(window_attrs).unwrap());

        // Initialize wgpu
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });

        let surface = instance.create_surface(window.clone()).unwrap();

        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }))
        .unwrap();

        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            ..Default::default()
        }))
        .unwrap();

        let size = window.inner_size();
        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface.get_capabilities(&adapter).formats[0],
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        surface.configure(&device, &surface_config);

        // Create shadow renderer
        let shadow_config = CascadeShadowConfig::default();
        let shadow_renderer = ShadowRenderer::new(&device, shadow_config);

        println!("🎮 Bevy Shadow Demo Initialized");
        println!("   Resolution: {}×{}", size.width, size.height);
        println!("   Backend: {:?}", adapter.get_info().backend);
        println!("   Device: {}", adapter.get_info().name);

        self.window = Some(window);
        self.device = Some(device);
        self.queue = Some(queue);
        self.surface = Some(surface);
        self.surface_config = Some(surface_config);
        self.shadow_renderer = Some(shadow_renderer);

        // Setup scene
        self.setup_scene();

        // Create render resources
        self.create_render_resources();
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                println!("👋 Closing demo...");
                event_loop.exit();
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(key_code),
                        state: ElementState::Pressed,
                        ..
                    },
                ..
            } => match key_code {
                KeyCode::Escape => event_loop.exit(),
                KeyCode::ArrowLeft => self.camera_yaw -= 0.1,
                KeyCode::ArrowRight => self.camera_yaw += 0.1,
                KeyCode::ArrowUp => self.camera_pitch = (self.camera_pitch + 0.1).min(1.5),
                KeyCode::ArrowDown => self.camera_pitch = (self.camera_pitch - 0.1).max(-1.5),
                KeyCode::KeyW => {
                    self.translate_camera(self.camera_forward_vec() * 0.5);
                }
                KeyCode::KeyS => {
                    self.translate_camera(self.camera_forward_vec() * -0.5);
                }
                KeyCode::KeyA => {
                    self.translate_camera(self.camera_right_vec() * -0.5);
                }
                KeyCode::KeyD => {
                    self.translate_camera(self.camera_right_vec() * 0.5);
                }
                KeyCode::KeyQ => {
                    self.translate_camera(self.camera_up_vec() * -0.5);
                }
                KeyCode::KeyE => {
                    self.translate_camera(self.camera_up_vec() * 0.5);
                }
                _ => {}
            },
            WindowEvent::RedrawRequested => {
                if let Err(e) = self.update_and_render() {
                    eprintln!("❌ Render error: {}", e);
                }
                self.window.as_ref().unwrap().request_redraw();
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}

fn main() -> Result<()> {
    env_logger::init();

    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = DemoApp::default();
    event_loop.run_app(&mut app)?;

    Ok(())
}
