// CSM Visual Validation Demo
//
// Controls:
//   WASD + Mouse: Camera movement
//   C: Toggle cascade visualization
//   S: Toggle shadows on/off
//   +/-: Zoom in/out
//
// Validates:
//   - Shadow rendering quality (PCF soft edges)
//   - Cascade transitions (near to far)
//   - Debug visualization (cascade colors)

use anyhow::Result;
use astraweave_render::shadow_csm::CsmRenderer;
use glam::{Mat4, Vec3};
use std::sync::Arc;
use std::time::Instant;
use wgpu::util::DeviceExt;
use winit::{
    application::ApplicationHandler,
    event::*,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::*,
    window::{Window, WindowId},
};

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 3],
    normal: [f32; 3],
}

impl Vertex {
    const ATTRIBS: [wgpu::VertexAttribute; 2] =
        wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3];

    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct CameraUniform {
    view_proj: [[f32; 4]; 4],
    view_pos: [f32; 3],
    _padding: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct LightUniform {
    direction: [f32; 3],
    _padding1: f32,
    color: [f32; 3],
    _padding2: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct RenderSettings {
    enable_shadows: u32,
    show_cascade_colors: u32,
    debug_mode: u32, // 0=normal, 1=UVs, 2=receiver_depth, 3=atlas_depth, 4=raw_sample
    _padding: u32,
}

struct Camera {
    position: Vec3,
    yaw: f32,
    pitch: f32,
    /// Vertical field of view in **radians**. Renamed from `fov` to `fovy`
    /// in Unified Camera campaign sub-phase C.6.C per C.5 audit L.5.8;
    /// the post-C.4.B field-name discipline (`fovy` for radians) now
    /// applies to this example's bespoke camera too.
    fovy: f32,
    near: f32,
    far: f32,
}

impl Camera {
    fn new() -> Self {
        Self {
            position: Vec3::new(0.0, 5.0, 10.0),
            yaw: -90.0_f32.to_radians(),
            pitch: -20.0_f32.to_radians(),
            fovy: 60.0_f32.to_radians(),
            near: 0.1,
            far: 100.0,
        }
    }

    fn forward(&self) -> Vec3 {
        Vec3::new(
            self.yaw.cos() * self.pitch.cos(),
            self.pitch.sin(),
            self.yaw.sin() * self.pitch.cos(),
        )
        .normalize()
    }

    fn right(&self) -> Vec3 {
        self.forward().cross(Vec3::Y).normalize()
    }

    fn view_matrix(&self) -> Mat4 {
        Mat4::look_at_rh(self.position, self.position + self.forward(), Vec3::Y)
    }

    fn proj_matrix(&self, aspect: f32) -> Mat4 {
        Mat4::perspective_rh(self.fovy, aspect, self.near, self.far)
    }
}

struct Scene {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    index_count: u32,
}

impl Scene {
    fn new(device: &wgpu::Device) -> Self {
        let (vertices, indices) = Self::generate_scene_geometry();

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Scene Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Scene Index Buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        Self {
            vertex_buffer,
            index_buffer,
            index_count: indices.len() as u32,
        }
    }

    fn generate_scene_geometry() -> (Vec<Vertex>, Vec<u32>) {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        // Ground plane (20x20) - FIXED: Correct CCW winding when viewed from above
        let ground_y = 0.0;
        let ground_size = 20.0;
        let base_idx = vertices.len() as u32;
        vertices.push(Vertex {
            position: [-ground_size, ground_y, -ground_size],
            normal: [0.0, 1.0, 0.0],
        });
        vertices.push(Vertex {
            position: [ground_size, ground_y, -ground_size],
            normal: [0.0, 1.0, 0.0],
        });
        vertices.push(Vertex {
            position: [ground_size, ground_y, ground_size],
            normal: [0.0, 1.0, 0.0],
        });
        vertices.push(Vertex {
            position: [-ground_size, ground_y, ground_size],
            normal: [0.0, 1.0, 0.0],
        });
        // CCW winding when viewed from above: 0-2-1, 0-3-2 (reversed from before)
        indices.extend_from_slice(&[base_idx, base_idx + 2, base_idx + 1]);
        indices.extend_from_slice(&[base_idx, base_idx + 3, base_idx + 2]);

        // Cubes sitting ON ground (center Y = size/2 so bottom touches Y=0)
        Self::add_cube(&mut vertices, &mut indices, Vec3::new(0.0, 0.5, 0.0), 1.0); // Center cube

        // Corner cubes at various distances
        Self::add_cube(&mut vertices, &mut indices, Vec3::new(-5.0, 0.4, -5.0), 0.8);
        Self::add_cube(&mut vertices, &mut indices, Vec3::new(5.0, 0.4, -5.0), 0.8);
        Self::add_cube(&mut vertices, &mut indices, Vec3::new(-5.0, 0.4, 5.0), 0.8);
        Self::add_cube(&mut vertices, &mut indices, Vec3::new(5.0, 0.4, 5.0), 0.8);

        // Far cubes for cascade testing (sitting on ground)
        Self::add_cube(&mut vertices, &mut indices, Vec3::new(0.0, 0.3, -15.0), 0.6);
        Self::add_cube(&mut vertices, &mut indices, Vec3::new(0.0, 0.3, -25.0), 0.6);

        (vertices, indices)
    }

    fn add_cube(vertices: &mut Vec<Vertex>, indices: &mut Vec<u32>, center: Vec3, size: f32) {
        let s = size / 2.0;
        let base_idx = vertices.len() as u32;

        // Cube vertices with normals
        let cube_data = [
            // Front face (Z+)
            ([center.x - s, center.y - s, center.z + s], [0.0, 0.0, 1.0]),
            ([center.x + s, center.y - s, center.z + s], [0.0, 0.0, 1.0]),
            ([center.x + s, center.y + s, center.z + s], [0.0, 0.0, 1.0]),
            ([center.x - s, center.y + s, center.z + s], [0.0, 0.0, 1.0]),
            // Back face (Z-)
            ([center.x + s, center.y - s, center.z - s], [0.0, 0.0, -1.0]),
            ([center.x - s, center.y - s, center.z - s], [0.0, 0.0, -1.0]),
            ([center.x - s, center.y + s, center.z - s], [0.0, 0.0, -1.0]),
            ([center.x + s, center.y + s, center.z - s], [0.0, 0.0, -1.0]),
            // Top face (Y+)
            ([center.x - s, center.y + s, center.z + s], [0.0, 1.0, 0.0]),
            ([center.x + s, center.y + s, center.z + s], [0.0, 1.0, 0.0]),
            ([center.x + s, center.y + s, center.z - s], [0.0, 1.0, 0.0]),
            ([center.x - s, center.y + s, center.z - s], [0.0, 1.0, 0.0]),
            // Bottom face (Y-)
            ([center.x - s, center.y - s, center.z - s], [0.0, -1.0, 0.0]),
            ([center.x + s, center.y - s, center.z - s], [0.0, -1.0, 0.0]),
            ([center.x + s, center.y - s, center.z + s], [0.0, -1.0, 0.0]),
            ([center.x - s, center.y - s, center.z + s], [0.0, -1.0, 0.0]),
            // Right face (X+)
            ([center.x + s, center.y - s, center.z + s], [1.0, 0.0, 0.0]),
            ([center.x + s, center.y - s, center.z - s], [1.0, 0.0, 0.0]),
            ([center.x + s, center.y + s, center.z - s], [1.0, 0.0, 0.0]),
            ([center.x + s, center.y + s, center.z + s], [1.0, 0.0, 0.0]),
            // Left face (X-)
            ([center.x - s, center.y - s, center.z - s], [-1.0, 0.0, 0.0]),
            ([center.x - s, center.y - s, center.z + s], [-1.0, 0.0, 0.0]),
            ([center.x - s, center.y + s, center.z + s], [-1.0, 0.0, 0.0]),
            ([center.x - s, center.y + s, center.z - s], [-1.0, 0.0, 0.0]),
        ];

        for (pos, normal) in cube_data.iter() {
            vertices.push(Vertex {
                position: *pos,
                normal: *normal,
            });
        }

        // Cube indices (6 faces × 2 triangles × 3 vertices = 36 indices)
        let face_indices = [
            0, 1, 2, 0, 2, 3, // Front
            4, 5, 6, 4, 6, 7, // Back
            8, 9, 10, 8, 10, 11, // Top
            12, 13, 14, 12, 14, 15, // Bottom
            16, 17, 18, 16, 18, 19, // Right
            20, 21, 22, 20, 22, 23, // Left
        ];

        indices.extend(face_indices.iter().map(|i| base_idx + i));
    }
}

struct App {
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: wgpu::Surface<'static>,
    surface_config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,

    camera: Camera,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,

    _light_buffer: wgpu::Buffer,
    light_bind_group: wgpu::BindGroup,

    settings_buffer: wgpu::Buffer,
    settings_bind_group: wgpu::BindGroup,

    csm: CsmRenderer,
    scene: Scene,
    main_pipeline: wgpu::RenderPipeline,

    // State
    enable_shadows: bool,
    show_cascade_colors: bool,
    debug_mode: u32,     // NEW: Shadow debug visualization mode
    movement: [bool; 6], // W, A, S, D, Space, Shift
    mouse_delta: (f32, f32),
    mouse_pressed: bool,
    last_frame: Instant,
    frame_count: u32,
}

impl App {
    fn new(window: std::sync::Arc<winit::window::Window>) -> Result<Self> {
        let size = window.inner_size();

        // Create wgpu instance
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let surface = instance.create_surface(window.clone())?;

        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }))?;

        let (device, queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                label: Some("CSM Demo Device"),
                memory_hints: Default::default(),
                trace: Default::default(),
            }))?;

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        surface.configure(&device, &surface_config);

        // Create camera
        let camera = Camera::new();
        let camera_uniform = CameraUniform {
            view_proj: Mat4::IDENTITY.to_cols_array_2d(),
            view_pos: camera.position.to_array(),
            _padding: 0.0,
        };

        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&[camera_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Camera Bind Group Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Camera Bind Group"),
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });

        // Create light (directional, from upper-left)
        let light_direction = Vec3::new(-0.5, -1.0, -0.3).normalize();
        let light_uniform = LightUniform {
            direction: light_direction.to_array(),
            _padding1: 0.0,
            color: [1.0, 0.95, 0.9],
            _padding2: 0.0,
        };

        let light_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Light Buffer"),
            contents: bytemuck::cast_slice(&[light_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let light_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Light Bind Group Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let light_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Light Bind Group"),
            layout: &light_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: light_buffer.as_entire_binding(),
            }],
        });

        // Create render settings buffer
        let settings_uniform = RenderSettings {
            enable_shadows: 1,
            show_cascade_colors: 0,
            debug_mode: 0,
            _padding: 0,
        };

        let settings_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Settings Buffer"),
            contents: bytemuck::cast_slice(&[settings_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let settings_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Settings Bind Group Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let settings_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Settings Bind Group"),
            layout: &settings_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: settings_buffer.as_entire_binding(),
            }],
        });

        // Create CSM renderer
        let mut csm = CsmRenderer::new(&device)?;

        // Initialize CSM with default cascades and upload to GPU
        let initial_view = Mat4::IDENTITY;
        let initial_proj = Mat4::perspective_rh(60.0_f32.to_radians(), 1.78, 0.1, 100.0);
        let light_direction = Vec3::new(-0.5, -1.0, -0.3).normalize();
        csm.update_cascades(
            Vec3::new(0.0, 5.0, 10.0),
            initial_view,
            initial_proj,
            light_direction,
            0.1,
            100.0,
        );
        csm.upload_to_gpu(&queue, &device);

        // Create scene geometry
        let scene = Scene::new(&device);

        // Create main render pipeline
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("CSM Demo Shader"),
            source: wgpu::ShaderSource::Wgsl(MAIN_SHADER.into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("CSM Demo Pipeline Layout"),
            bind_group_layouts: &[
                &camera_bind_group_layout,
                &light_bind_group_layout,
                &csm.bind_group_layout,
                &settings_bind_group_layout,
            ],
            push_constant_ranges: &[],
        });

        let main_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("CSM Demo Main Pipeline"),
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
                    blend: Some(wgpu::BlendState::REPLACE),
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
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        Ok(Self {
            device,
            queue,
            surface,
            surface_config,
            size,
            camera,
            camera_buffer,
            camera_bind_group,
            _light_buffer: light_buffer,
            light_bind_group,
            settings_buffer,
            settings_bind_group,
            csm,
            scene,
            main_pipeline,
            enable_shadows: true,
            show_cascade_colors: false,
            debug_mode: 0,
            movement: [false; 6],
            mouse_delta: (0.0, 0.0),
            mouse_pressed: false,
            last_frame: Instant::now(),
            frame_count: 0,
        })
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.surface_config.width = new_size.width;
            self.surface_config.height = new_size.height;
            self.surface.configure(&self.device, &self.surface_config);
        }
    }

    fn update(&mut self) {
        let dt = self.last_frame.elapsed().as_secs_f32();
        self.last_frame = Instant::now();

        // Update camera from input
        let speed = 5.0 * dt;
        let forward = self.camera.forward();
        let right = self.camera.right();

        if self.movement[0] {
            self.camera.position += forward * speed;
        } // W
        if self.movement[2] {
            self.camera.position -= forward * speed;
        } // S
        if self.movement[1] {
            self.camera.position -= right * speed;
        } // A
        if self.movement[3] {
            self.camera.position += right * speed;
        } // D
        if self.movement[4] {
            self.camera.position.y += speed;
        } // Space
        if self.movement[5] {
            self.camera.position.y -= speed;
        } // Shift

        // Mouse look
        if self.mouse_pressed {
            let sensitivity = 0.003;
            self.camera.yaw += self.mouse_delta.0 * sensitivity;
            self.camera.pitch -= self.mouse_delta.1 * sensitivity;
            self.camera.pitch = self
                .camera
                .pitch
                .clamp(-89.0_f32.to_radians(), 89.0_f32.to_radians());
        }
        self.mouse_delta = (0.0, 0.0);

        // Update camera uniform
        // C.6.C: `.max(0.01)` aspect guard per CAMERA_CONVENTIONS.md §2.3,
        // matching the canonical pipeline's discipline at
        // `astraweave-camera/src/freefly.rs:48`. Prevents NaN propagation
        // if aspect somehow reaches zero (resize handler already guards
        // `width > 0 && height > 0` at the boundary but this is
        // defense-in-depth).
        let aspect = (self.size.width as f32 / self.size.height as f32).max(0.01);
        let view = self.camera.view_matrix();
        let proj = self.camera.proj_matrix(aspect);
        let view_proj = proj * view;

        let camera_uniform = CameraUniform {
            view_proj: view_proj.to_cols_array_2d(),
            view_pos: self.camera.position.to_array(),
            _padding: 0.0,
        };

        self.queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&[camera_uniform]),
        );

        // Update render settings
        let settings_uniform = RenderSettings {
            enable_shadows: if self.enable_shadows { 1 } else { 0 },
            show_cascade_colors: if self.show_cascade_colors { 1 } else { 0 },
            debug_mode: self.debug_mode,
            _padding: 0,
        };
        self.queue.write_buffer(
            &self.settings_buffer,
            0,
            bytemuck::cast_slice(&[settings_uniform]),
        );

        // Update CSM cascades
        let light_direction = Vec3::new(-0.5, -1.0, -0.3).normalize();
        self.csm.update_cascades(
            self.camera.position,
            view,
            proj,
            light_direction,
            self.camera.near,
            self.camera.far,
        );
        self.csm.upload_to_gpu(&self.queue, &self.device);
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        self.frame_count += 1;

        if self.frame_count == 1 {
            println!("🎬 First frame render:");
            println!("  - Shadows enabled: {}", self.enable_shadows);
            println!("  - Cascade viz: {}", self.show_cascade_colors);
            println!("  - Scene index count: {}", self.scene.index_count);
            println!("  - Camera position: {:?}", self.camera.position);
            println!(
                "  - Shadow atlas: {}x{} (4 cascades)",
                astraweave_render::shadow_csm::ATLAS_RESOLUTION,
                astraweave_render::shadow_csm::ATLAS_RESOLUTION
            );
            println!(
                "  - Cascade resolution: {}x{}",
                astraweave_render::shadow_csm::CASCADE_RESOLUTION,
                astraweave_render::shadow_csm::CASCADE_RESOLUTION
            );
            println!();
            println!("🐛 Debug controls:");
            println!("  V = Cycle debug modes (UVs/depth/raw sample)");
            println!("  C = Toggle cascade colors");
            println!("  X = Toggle shadows on/off");
        }

        let output = self.surface.get_current_texture()?;
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor {
            usage: None,
            ..Default::default()
        });

        // Create depth texture
        let depth_texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Depth Texture"),
            size: wgpu::Extent3d {
                width: self.size.width,
                height: self.size.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });

        let depth_view = depth_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        // 1. Render shadow maps
        if self.frame_count == 1 {
            println!("🔧 Rendering shadow maps (frame 1):");
            println!("   - Index count: {}", self.scene.index_count);
            println!("   - Cascades: 4");
        }
        self.csm.render_shadow_maps(
            &mut encoder,
            &self.scene.vertex_buffer,
            &self.scene.index_buffer,
            self.scene.index_count,
        );

        // 2. Main render pass
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Main Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            render_pass.set_pipeline(&self.main_pipeline);
            render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
            render_pass.set_bind_group(1, &self.light_bind_group, &[]);
            render_pass.set_bind_group(2, self.csm.bind_group.as_ref().unwrap(), &[]);
            render_pass.set_bind_group(3, &self.settings_bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.scene.vertex_buffer.slice(..));
            render_pass
                .set_index_buffer(self.scene.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            render_pass.draw_indexed(0..self.scene.index_count, 0, 0..1);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }

    fn input(&mut self, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(key),
                        state,
                        ..
                    },
                ..
            } => {
                let pressed = *state == ElementState::Pressed;
                match key {
                    KeyCode::KeyW => self.movement[0] = pressed,
                    KeyCode::KeyA => self.movement[1] = pressed,
                    KeyCode::KeyS => self.movement[2] = pressed,
                    KeyCode::KeyD => self.movement[3] = pressed,
                    KeyCode::Space => self.movement[4] = pressed,
                    KeyCode::ShiftLeft => self.movement[5] = pressed,
                    KeyCode::KeyC if pressed => {
                        self.show_cascade_colors = !self.show_cascade_colors;
                        println!("Cascade visualization: {}", self.show_cascade_colors);
                    }
                    KeyCode::KeyX if pressed => {
                        self.enable_shadows = !self.enable_shadows;
                        println!("Shadows: {}", self.enable_shadows);
                    }
                    KeyCode::KeyV if pressed => {
                        self.debug_mode = (self.debug_mode + 1) % 6; // Updated to 6 modes
                        let mode_name = match self.debug_mode {
                            0 => "Normal rendering",
                            1 => "Shadow UVs (red=X, green=Y)",
                            2 => "Receiver depth (lighter=farther from light)",
                            3 => "Shadow atlas depth (what's IN the texture)",
                            4 => "Raw shadow sample (white=lit, black=shadow)",
                            5 => "Direct atlas view (screen-space, shows if atlas has geometry)",
                            _ => "Unknown",
                        };
                        println!("🐛 Debug mode {}: {}", self.debug_mode, mode_name);
                    }
                    _ => return false,
                }
                true
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if *button == winit::event::MouseButton::Left {
                    self.mouse_pressed = *state == ElementState::Pressed;
                    true
                } else {
                    false
                }
            }
            WindowEvent::CursorMoved { .. } => false,
            _ => false,
        }
    }

    fn mouse_motion(&mut self, delta: (f64, f64)) {
        self.mouse_delta.0 += delta.0 as f32;
        self.mouse_delta.1 += delta.1 as f32;
    }
}

const MAIN_SHADER: &str = r#"
struct CameraUniform {
    view_proj: mat4x4<f32>,
    view_pos: vec3<f32>,
}

struct LightUniform {
    direction: vec3<f32>,
    color: vec3<f32>,
}

struct RenderSettings {
    enable_shadows: u32,
    show_cascade_colors: u32,
    debug_mode: u32,  // 0=normal, 1=UVs, 2=receiver_depth, 3=atlas_depth, 4=raw_sample
}

struct ShadowCascade {
    view_proj: mat4x4<f32>,
    split_distances: vec4<f32>,
    atlas_transform: vec4<f32>, // (offset_x, offset_y, scale_x, scale_y)
}

@group(0) @binding(0) var<uniform> camera: CameraUniform;
@group(1) @binding(0) var<uniform> light: LightUniform;
@group(2) @binding(0) var shadow_atlas: texture_depth_2d_array;
@group(2) @binding(1) var shadow_sampler: sampler_comparison;
@group(2) @binding(2) var<uniform> cascades: array<ShadowCascade, 4>;
@group(3) @binding(0) var<uniform> settings: RenderSettings;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) view_depth: f32,
}

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.world_position = in.position;
    out.normal = in.normal;
    
    let view_pos = camera.view_proj * vec4<f32>(in.position, 1.0);
    out.view_depth = -view_pos.z; // Positive depth in view space
    out.clip_position = view_pos;
    
    return out;
}

// 🎯 BEVY INTEGRATION: Use proven shadow math from Bevy Engine
// Get cascade index based on view-space depth
fn get_cascade_index(view_z: f32) -> u32 {
    for (var i: u32 = 0u; i < 4u; i = i + 1u) {
        if (-view_z < cascades[i].split_distances.y) {
            return i;
        }
    }
    return 4u;
}

// Converts from world space to the uv position in the light's shadow map.
// Based on Bevy's world_to_directional_light_local
fn world_to_shadow_uv(world_pos: vec3<f32>, cascade_index: u32) -> vec4<f32> {
    let cascade = &cascades[cascade_index];
    
    let offset_position_clip = (*cascade).view_proj * vec4<f32>(world_pos, 1.0);
    if (offset_position_clip.w <= 0.0) {
        return vec4(0.0, 0.0, 0.0, 0.0); // Outside shadow map
    }
    let offset_position_ndc = offset_position_clip.xyz / offset_position_clip.w;
    
    // No shadow outside the orthographic projection volume
    if (any(offset_position_ndc.xy < vec2<f32>(-1.0)) || offset_position_ndc.z < 0.0
            || any(offset_position_ndc > vec3<f32>(1.0))) {
        return vec4(0.0, 0.0, 0.0, 0.0);
    }

    // Compute texture coordinates for shadow lookup, compensating for the Y-flip difference
    // between the NDC and texture coordinates (BEVY'S PROVEN FORMULA)
    let flip_correction = vec2<f32>(0.5, -0.5);
    let light_local = offset_position_ndc.xy * flip_correction + vec2<f32>(0.5, 0.5);

    let depth = offset_position_ndc.z;

    return vec4(light_local, depth, 1.0); // w=1.0 means valid sample
}

// 🎯 BEVY-STYLE PCF sampling (adapted to vec4 shadow_uv)
fn sample_shadow_pcf(shadow_uv: vec4<f32>, cascade_idx: u32, bias: f32) -> f32 {
    // If shadow_uv.w == 0, we're outside the shadow map
    if (shadow_uv.w == 0.0) {
        return 1.0; // Fully lit (no shadow)
    }
    
    let atlas_size = 2048.0;  // Per-cascade resolution
    let texel_size = 1.0 / atlas_size;
    let biased_depth = shadow_uv.z - bias;
    
    var shadow_factor = 0.0;
    for (var x = -2; x <= 2; x++) {
        for (var y = -2; y <= 2; y++) {
            let offset = vec2<f32>(f32(x), f32(y)) * texel_size;
            shadow_factor += textureSampleCompareLevel(
                shadow_atlas,
                shadow_sampler,
                shadow_uv.xy + offset,
                i32(cascade_idx),  // Array layer
                biased_depth
            );
        }
    }
    return shadow_factor / 25.0;
}

fn sample_shadow_csm(world_pos: vec3<f32>, view_depth: f32, normal: vec3<f32>) -> f32 {
    let cascade_idx = get_cascade_index(view_depth);
    if (cascade_idx >= 4u) {
        return 1.0; // Outside all cascades
    }
    
    let shadow_uv = world_to_shadow_uv(world_pos, cascade_idx);
    
    // Adaptive slope-dependent bias
    let ndotl = max(dot(normal, -light.direction), 0.0);
    let base_bias = 0.002;
    let slope_scale = 0.005;
    let bias = base_bias + slope_scale * sqrt(1.0 - ndotl * ndotl);
    
    return sample_shadow_pcf(shadow_uv, cascade_idx, bias);
}

fn debug_cascade_color(view_depth: f32) -> vec3<f32> {
    let idx = get_cascade_index(view_depth);
    switch idx {
        case 0u: { return vec3<f32>(1.0, 0.0, 0.0); }
        case 1u: { return vec3<f32>(0.0, 1.0, 0.0); }
        case 2u: { return vec3<f32>(0.0, 0.0, 1.0); }
        default: { return vec3<f32>(1.0, 1.0, 0.0); }
    }
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let normal = normalize(in.normal);
    let light_dir = normalize(-light.direction);
    
    // Basic diffuse lighting
    let diffuse = max(dot(normal, light_dir), 0.0);
    
    // Shadow sampling with DEBUG visualization
    var shadow_factor = 1.0;
    
    if settings.enable_shadows != 0u {
        let cascade_idx = get_cascade_index(in.view_depth);
        let shadow_uv = world_to_shadow_uv(in.world_position, cascade_idx);
        
        // DEBUG MODE 1: Visualize shadow UVs + validity
        if settings.debug_mode == 1u {
            // Show UVs even if invalid (don't return magenta)
            // Red/Green = UV coordinates, Blue = cascade index / 3
            return vec4<f32>(shadow_uv.x, shadow_uv.y, f32(cascade_idx) / 3.0, 1.0);
        }
        
        // DEBUG MODE 2: Visualize receiver depth (fragment's depth in light space)
        if settings.debug_mode == 2u {
            // Show depth even if invalid
            let depth_vis = shadow_uv.z; // Depth in NDC space
            return vec4<f32>(depth_vis, depth_vis, depth_vis, 1.0);
        }
        
        // DEBUG MODE 3: Sample shadow atlas depth directly (what's stored in texture)
        if settings.debug_mode == 3u {
            // Clamp UVs to [0,1] to avoid sampling outside texture
            let clamped_uv = clamp(shadow_uv.xy, vec2<f32>(0.0), vec2<f32>(1.0));
            let atlas_depth = textureLoad(shadow_atlas, vec2<i32>(clamped_uv * 2048.0), i32(cascade_idx), 0);
            return vec4<f32>(atlas_depth, atlas_depth, atlas_depth, 1.0);
        }
        
        // DEBUG MODE 4: Show depth comparison values
        if settings.debug_mode == 4u {
            // Clamp UVs to avoid sampling outside
            let clamped_uv = clamp(shadow_uv.xy, vec2<f32>(0.0), vec2<f32>(1.0));
            let atlas_depth = textureLoad(shadow_atlas, vec2<i32>(clamped_uv * 2048.0), i32(cascade_idx), 0);
            let fragment_depth = shadow_uv.z;
            
            // Red = fragment depth, Green = atlas depth, Blue = comparison result
            let in_shadow = f32(fragment_depth > atlas_depth);
            return vec4<f32>(fragment_depth, atlas_depth, in_shadow, 1.0);
        }
        
        // DEBUG MODE 5: Direct shadow atlas visualization (NO coordinate transform)
        if settings.debug_mode == 5u {
            // Sample entire atlas as if it were a fullscreen quad
            let frag_uv = vec2<f32>(in.clip_position.x / 800.0, in.clip_position.y / 600.0);
            let clamped_uv = clamp(frag_uv, vec2<f32>(0.0), vec2<f32>(1.0));
            let atlas_coord = vec2<i32>(clamped_uv * 2048.0);
            
            // Show cascade 0 (closest)
            let atlas_depth = textureLoad(shadow_atlas, atlas_coord, 0, 0);
            let boosted = pow(atlas_depth, 0.5); // Gamma correction for visibility
            return vec4<f32>(boosted, boosted, boosted, 1.0);
        }
        
        // Normal shadow sampling with PCF
        shadow_factor = sample_shadow_csm(in.world_position, in.view_depth, normal);
    }
    
    // Base color - make ground darker to show shadows better
    var base_color = vec3<f32>(0.5, 0.5, 0.5);
    
    // Apply lighting with stronger shadow contrast
    let ambient = 0.15;  // Lower ambient = darker shadows
    let lit = ambient + shadow_factor * diffuse * 0.85;
    var final_color = base_color * lit * light.color;
    
    // Cascade visualization (check if enabled)
    if settings.show_cascade_colors != 0u {
        let cascade_color = debug_cascade_color(in.view_depth);
        final_color = mix(final_color, cascade_color, 0.5);
    }
    
    return vec4<f32>(final_color, 1.0);
}
"#;

struct DemoApp {
    app: Option<App>,
    window: Option<Arc<Window>>,
}

impl ApplicationHandler for DemoApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let window_attributes = Window::default_attributes()
                .with_title("CSM Shadow Demo - WASD+Mouse to move, C=Cascades, S=Shadows")
                .with_inner_size(winit::dpi::LogicalSize::new(1280, 720));
            let window = Arc::new(event_loop.create_window(window_attributes).unwrap());
            self.window = Some(window.clone());

            match App::new(window) {
                Ok(app) => {
                    println!("CSM Demo Controls:");
                    println!("  WASD + Mouse: Camera movement");
                    println!("  C: Toggle cascade visualization");
                    println!("  S: Toggle shadows on/off");
                    self.app = Some(app);
                }
                Err(e) => {
                    eprintln!("Failed to create app: {}", e);
                    event_loop.exit();
                }
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        if Some(window_id) != self.window.as_ref().map(|w| w.id()) {
            return;
        }

        if let Some(app) = &mut self.app {
            if !app.input(&event) {
                match event {
                    WindowEvent::CloseRequested => event_loop.exit(),
                    WindowEvent::Resized(physical_size) => {
                        app.resize(physical_size);
                    }
                    WindowEvent::RedrawRequested => {
                        app.update();
                        match app.render() {
                            Ok(_) => {}
                            Err(wgpu::SurfaceError::Lost) => app.resize(app.size),
                            Err(wgpu::SurfaceError::OutOfMemory) => event_loop.exit(),
                            Err(e) => eprintln!("Render error: {:?}", e),
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device_id: winit::event::DeviceId,
        event: DeviceEvent,
    ) {
        if let DeviceEvent::MouseMotion { delta } = event {
            if let Some(app) = &mut self.app {
                app.mouse_motion(delta);
            }
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

    let mut demo_app = DemoApp {
        app: None,
        window: None,
    };

    event_loop.run_app(&mut demo_app)?;

    Ok(())
}
