use glam::{Mat4, Quat, Vec3};
use std::io::Write;
use std::sync::Arc;
use wgpu::util::DeviceExt;
use winit::{
    application::ApplicationHandler,
    event::*,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId},
};

mod gltf_loader;

// Shadow Shader - Vertex only for depth rendering
const SHADOW_SHADER: &str = r#"
struct LightUniforms {
    view_proj: mat4x4<f32>,
    position: vec3<f32>,
    color: vec3<f32>,
}

struct ModelUniforms {
    model: mat4x4<f32>,
}

@group(0) @binding(0) var<uniform> light: LightUniforms;
@group(1) @binding(0) var<uniform> model: ModelUniforms;

struct VertexInput {
    @location(0) position: vec3<f32>,
}

@vertex
fn vs_main(in: VertexInput) -> @builtin(position) vec4<f32> {
    return light.view_proj * model.model * vec4<f32>(in.position, 1.0);
}
"#;

// ============================================================================
// CORE DATA STRUCTURES
// ============================================================================

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 3],
    normal: [f32; 3],
    uv: [f32; 2],
    color: [f32; 4],
    tangent: [f32; 4],
}

impl Vertex {
    const ATTRIBS: [wgpu::VertexAttribute; 5] = wgpu::vertex_attr_array![
        0 => Float32x3,
        1 => Float32x3,
        2 => Float32x2,
        3 => Float32x4,
        4 => Float32x4,
    ];

    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct CameraUniforms {
    view_proj: [[f32; 4]; 4],
    camera_pos: [f32; 3],
    _padding: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct LightUniforms {
    view_proj: [[f32; 4]; 4],
    position: [f32; 3],
    _padding: f32,
    color: [f32; 3],
    _padding2: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct ModelUniforms {
    model: [[f32; 4]; 4],
}

struct Material {
    #[allow(dead_code)]
    name: String,
    bind_group: wgpu::BindGroup,
}

struct Mesh {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    num_indices: u32,
    material_index: usize,
}

struct SceneObject {
    mesh_index: usize,
    #[allow(dead_code)]
    position: Vec3,
    #[allow(dead_code)]
    rotation: Quat,
    #[allow(dead_code)]
    scale: Vec3,
    model_bind_group: wgpu::BindGroup,
}

// ============================================================================
// APPLICATION STATE
// ============================================================================

struct ShowcaseApp {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,

    render_pipeline: wgpu::RenderPipeline,
    sky_pipeline: wgpu::RenderPipeline,
    terrain_pipeline: wgpu::RenderPipeline,
    shadow_pipeline: wgpu::RenderPipeline,

    material_layout: wgpu::BindGroupLayout,
    model_layout: wgpu::BindGroupLayout,
    terrain_layout: wgpu::BindGroupLayout,

    depth_texture: wgpu::TextureView,
    msaa_texture: wgpu::TextureView,
    _shadow_texture: wgpu::Texture,
    shadow_view: wgpu::TextureView,
    _shadow_sampler: wgpu::Sampler,

    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    camera_pos: Vec3,
    camera_yaw: f32,
    camera_pitch: f32,

    #[allow(dead_code)]
    light_buffer: wgpu::Buffer,
    light_bind_group: wgpu::BindGroup,
    shadow_bind_group: wgpu::BindGroup,

    materials: Vec<Material>,
    meshes: Vec<Mesh>,
    objects: Vec<SceneObject>,

    sky_bind_group: wgpu::BindGroup,
    sky_mesh_index: usize,

    terrain_bind_group: wgpu::BindGroup,
    terrain_model_bind_group: wgpu::BindGroup,
    terrain_mesh_index: usize,

    // Material indices for GLTF models
    pine_bark_mat: usize,
    pine_leaves_mat: usize,
    tower_wood_mat: usize,
    tower_stone_mat: usize,

    mouse_pressed: bool,
}

impl ShowcaseApp {
    async fn new(window: Arc<Window>) -> Self {
        let size = window.inner_size();
        println!("Window size: {}x{}", size.width, size.height);
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });

        let surface = instance.create_surface(window.clone()).unwrap();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: Default::default(),
                trace: wgpu::Trace::Off,
            })
            .await
            .unwrap();

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        // Layouts
        let camera_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Camera Layout"),
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

        let light_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Light Layout"),
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
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Depth,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                    count: None,
                },
            ],
        });

        let shadow_uniforms_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Shadow Uniforms Layout"),
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

        let material_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Material Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let model_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Model Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        // Pipelines
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader V2"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader_v2.wgsl").into()),
        });

        let sky_shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Skybox Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("skybox.wgsl").into()),
        });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[
                    &camera_layout,
                    &light_layout,
                    &material_layout,
                    &model_layout,
                ],
                push_constant_ranges: &[],
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
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
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
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
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 4,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        let sky_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Sky Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let sky_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Sky Pipeline Layout"),
            bind_group_layouts: &[&camera_layout, &sky_layout],
            push_constant_ranges: &[],
        });

        let sky_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Sky Pipeline"),
            layout: Some(&sky_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &sky_shader_module,
                entry_point: Some("vs_skybox"),
                buffers: &[Vertex::desc()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &sky_shader_module,
                entry_point: Some("fs_skybox"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None, // Disable culling for skybox to ensure visibility
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: false,
                depth_compare: wgpu::CompareFunction::LessEqual,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 4,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        // Terrain Layout and Pipeline
        let terrain_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Terrain Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 5,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 6,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let terrain_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Terrain Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("terrain.wgsl").into()),
        });

        let terrain_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Terrain Pipeline Layout"),
                bind_group_layouts: &[
                    &camera_layout,
                    &light_layout,
                    &terrain_layout,
                    &model_layout,
                ],
                push_constant_ranges: &[],
            });

        let terrain_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Terrain Pipeline"),
            layout: Some(&terrain_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &terrain_shader,
                entry_point: Some("vs_main"),
                buffers: &[Vertex::desc()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &terrain_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
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
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 4,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        // Shadow Texture
        let shadow_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Shadow Texture"),
            size: wgpu::Extent3d {
                width: 2048,
                height: 2048,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let shadow_view = shadow_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let shadow_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Shadow Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            compare: Some(wgpu::CompareFunction::Less),
            ..Default::default()
        });

        // Shadow Pipeline
        let shadow_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shadow Shader"),
            source: wgpu::ShaderSource::Wgsl(SHADOW_SHADER.into()),
        });

        let shadow_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Shadow Pipeline Layout"),
                bind_group_layouts: &[&shadow_uniforms_layout, &model_layout],
                push_constant_ranges: &[],
            });

        let shadow_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Shadow Pipeline"),
            layout: Some(&shadow_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shadow_shader,
                entry_point: Some("vs_main"),
                buffers: &[Vertex::desc()],
                compilation_options: Default::default(),
            },
            fragment: None,
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState {
                    constant: 2,
                    slope_scale: 2.0,
                    clamp: 0.0,
                },
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        // Buffers
        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&[CameraUniforms {
                view_proj: Mat4::IDENTITY.to_cols_array_2d(),
                camera_pos: [0.0, 0.0, 0.0],
                _padding: 0.0,
            }]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Camera Bind Group"),
            layout: &camera_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });

        let light_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Light Buffer"),
            contents: bytemuck::cast_slice(&[LightUniforms {
                view_proj: Mat4::IDENTITY.to_cols_array_2d(),
                position: [50.0, 100.0, 50.0],
                _padding: 0.0,
                color: [1.2, 1.1, 1.0], // Warm sunlight
                _padding2: 0.0,
            }]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let light_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Light Bind Group"),
            layout: &light_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: light_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&shadow_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&shadow_sampler),
                },
            ],
        });

        let shadow_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Shadow Bind Group"),
            layout: &shadow_uniforms_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: light_buffer.as_entire_binding(),
            }],
        });

        // Skybox Texture - Try HDR first, fallback to PNG
        let sky_path_hdr = "assets/hdri/polyhaven/kloppenheim/kloppenheim_06_puresky_2k.hdr";
        let sky_path_png = "assets/sky_equirect.png";

        let (sky_texture, _sky_size) = if std::path::Path::new(sky_path_hdr).exists() {
            println!("Loading HDR skybox: {}", sky_path_hdr);
            let sky_img = image::open(sky_path_hdr)
                .expect("Failed to load HDR")
                .to_rgba8();
            let size = wgpu::Extent3d {
                width: sky_img.width(),
                height: sky_img.height(),
                depth_or_array_layers: 1,
            };
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("Sky Texture HDR"),
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
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
                &sky_img,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(4 * sky_img.width()),
                    rows_per_image: Some(sky_img.height()),
                },
                size,
            );
            (texture, size)
        } else {
            println!("HDR not found, using PNG skybox: {}", sky_path_png);
            let sky_img = image::open(sky_path_png)
                .expect("Missing sky_equirect.png")
                .to_rgba8();
            let size = wgpu::Extent3d {
                width: sky_img.width(),
                height: sky_img.height(),
                depth_or_array_layers: 1,
            };
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("Sky Texture"),
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
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
                &sky_img,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(4 * sky_img.width()),
                    rows_per_image: Some(sky_img.height()),
                },
                size,
            );
            (texture, size)
        };

        let sky_view = sky_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sky_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let sky_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Sky Bind Group"),
            layout: &sky_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&sky_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sky_sampler),
                },
            ],
        });

        // Textures
        let depth_texture = device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("Depth Texture"),
                size: wgpu::Extent3d {
                    width: config.width,
                    height: config.height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 4,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Depth32Float,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            })
            .create_view(&wgpu::TextureViewDescriptor::default());

        let msaa_texture = device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("MSAA Texture"),
                size: wgpu::Extent3d {
                    width: config.width,
                    height: config.height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 4,
                dimension: wgpu::TextureDimension::D2,
                format: config.format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            })
            .create_view(&wgpu::TextureViewDescriptor::default());

        // Load placeholder terrain textures for terrain bind group initialization
        // Helper macro to load and create texture
        let load_texture = |path: &str, label: &str| {
            let img = image::open(path)
                .unwrap_or_else(|_| panic!("Missing {}", path))
                .to_rgba8();
            let size = wgpu::Extent3d {
                width: img.width(),
                height: img.height(),
                depth_or_array_layers: 1,
            };
            let is_normal_or_rough = label.contains("Norm") || label.contains("Rough");
            let format = if is_normal_or_rough {
                wgpu::TextureFormat::Rgba8Unorm
            } else {
                wgpu::TextureFormat::Rgba8UnormSrgb
            };

            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some(label),
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
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
                &img,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(4 * img.width()),
                    rows_per_image: Some(img.height()),
                },
                size,
            );
            texture.create_view(&wgpu::TextureViewDescriptor::default())
        };

        let placeholder_grass_diff_view = load_texture(
            "assets/textures/pine forest textures/grass_medium_01_diff.png",
            "Placeholder Grass Diff",
        );
        let placeholder_grass_norm_view = load_texture(
            "assets/textures/pine forest textures/grass_medium_01_nor_gl.png",
            "Placeholder Grass Norm",
        );
        let placeholder_grass_rough_view = load_texture(
            "assets/textures/pine forest textures/grass_medium_01_rough.png",
            "Placeholder Grass Rough",
        );
        let placeholder_rock_diff_view = load_texture(
            "assets/textures/pine forest textures/rock_moss_set_01_diff.png",
            "Placeholder Rock Diff",
        );
        let placeholder_rock_norm_view = load_texture(
            "assets/textures/pine forest textures/rock_moss_set_01_nor_gl.png",
            "Placeholder Rock Norm",
        );
        let placeholder_rock_rough_view = load_texture(
            "assets/textures/pine forest textures/rock_moss_set_01_rough.png",
            "Placeholder Rock Rough",
        );

        let placeholder_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            ..Default::default()
        });

        let placeholder_terrain_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Placeholder Terrain Bind Group"),
            layout: &terrain_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&placeholder_grass_diff_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&placeholder_grass_norm_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&placeholder_grass_rough_view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&placeholder_rock_diff_view),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::TextureView(&placeholder_rock_norm_view),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: wgpu::BindingResource::TextureView(&placeholder_rock_rough_view),
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: wgpu::BindingResource::Sampler(&placeholder_sampler),
                },
            ],
        });

        let terrain_model_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Terrain Model Buffer"),
            contents: bytemuck::cast_slice(&[ModelUniforms {
                model: Mat4::IDENTITY.to_cols_array_2d(),
            }]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let terrain_model_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Terrain Model Bind Group"),
            layout: &model_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: terrain_model_buffer.as_entire_binding(),
            }],
        });

        let mut app = Self {
            window,
            surface,
            device,
            queue,
            config,
            render_pipeline,
            sky_pipeline,
            terrain_pipeline,
            shadow_pipeline,
            material_layout,
            model_layout,
            terrain_layout,
            depth_texture,
            msaa_texture,
            _shadow_texture: shadow_texture,
            shadow_view,
            _shadow_sampler: shadow_sampler,
            camera_buffer,
            camera_bind_group,
            terrain_model_bind_group,
            camera_pos: Vec3::new(0.0, 25.0, 60.0), // Elevated spawn point
            camera_yaw: 0.0,                        // Reset yaw
            camera_pitch: -0.1,                     // Look slightly down
            light_buffer,
            light_bind_group,
            shadow_bind_group,
            materials: Vec::new(),
            meshes: Vec::new(),
            objects: Vec::new(),
            sky_bind_group,
            sky_mesh_index: 0,
            terrain_bind_group: placeholder_terrain_bind_group,
            terrain_mesh_index: 0,
            pine_bark_mat: 0,
            pine_leaves_mat: 0,
            tower_wood_mat: 0,
            tower_stone_mat: 0,
            mouse_pressed: false,
        };

        app.init_scene();
        app
    }

    fn init_scene(&mut self) {
        println!("Initializing Floating Island Scene...");

        // Clear existing objects
        self.objects.clear();

        println!("=== MATERIAL CREATION START ===");
        println!("Materials vector size BEFORE: {}", self.materials.len());

        // 1. Materials
        println!("Loading materials...");
        println!("Loading Grass Mat...");
        std::io::stdout().flush().unwrap();
        let _grass_mat = self.create_material_from_texture(
            "Grass",
            "assets/textures/pine forest textures/grass_medium_01_diff.png",
        );
        println!("  -> Grass material index: {}", _grass_mat);
        println!("Loading Rock Mat...");
        std::io::stdout().flush().unwrap();
        let _rock_mat = self.create_material_from_texture(
            "Rock",
            "assets/textures/pine forest textures/rock_moss_set_01_diff.png",
        );
        println!("  -> Rock material index: {}", _rock_mat);
        let water_mat = self.create_material_from_color("Water", [0.0, 0.3, 0.7, 0.6]); // Deep blue, more transparent
        println!("  -> Water material index: {}", water_mat);

        // Materials for GLTF models
        println!("Loading PineBark Mat...");
        std::io::stdout().flush().unwrap();
        self.pine_bark_mat = self.create_material_from_texture(
            "PineBark",
            "assets/textures/pine forest textures/pine_bark_diff.png",
        );
        println!("  -> PineBark material index: {}", self.pine_bark_mat);
        println!("Loading PineLeaves Mat...");
        std::io::stdout().flush().unwrap();
        self.pine_leaves_mat = self.create_material_from_texture(
            "PineLeaves",
            "assets/textures/pine forest textures/pine_twig_diff.png",
        );
        println!("  -> PineLeaves material index: {}", self.pine_leaves_mat);
        println!("Loading TowerWood Mat...");
        std::io::stdout().flush().unwrap();
        self.tower_wood_mat = self.create_material_from_texture(
            "TowerWood",
            "assets/textures/pine forest textures/pine_bark_diff.png",
        );
        println!("  -> TowerWood material index: {}", self.tower_wood_mat);
        println!("Loading TowerStone Mat...");
        std::io::stdout().flush().unwrap();
        self.tower_stone_mat =
            self.create_material_from_texture("TowerStone", "assets/textures/cobblestone.png");
        println!("  -> TowerStone material index: {}", self.tower_stone_mat);

        println!("Materials vector size AFTER: {}", self.materials.len());
        println!("=== MATERIAL CREATION END ===\n");

        // 2. Floating Island Terrain (unified mesh for terrain pipeline)
        println!("Generating floating island terrain...");
        let terrain_mesh_idx = self.create_floating_island_terrain(50.0, 40.0, 32);
        println!("Terrain generation DONE");
        self.terrain_mesh_index = terrain_mesh_idx;

        // Create terrain bind group with all textures (diffuse, normal, roughness for grass and rock)
        println!("Starting Texture Load");
        let load_terrain_texture = |path: &str, label: &str| {
            let img = image::open(path)
                .unwrap_or_else(|_| panic!("Missing {}", path))
                .to_rgba8();
            let size = wgpu::Extent3d {
                width: img.width(),
                height: img.height(),
                depth_or_array_layers: 1,
            };

            let is_normal_or_rough = label.contains("Norm") || label.contains("Rough");
            let format = if is_normal_or_rough {
                wgpu::TextureFormat::Rgba8Unorm
            } else {
                wgpu::TextureFormat::Rgba8UnormSrgb
            };

            let texture = self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some(label),
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });
            self.queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                &img,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(4 * img.width()),
                    rows_per_image: Some(img.height()),
                },
                size,
            );
            texture.create_view(&wgpu::TextureViewDescriptor::default())
        };

        let grass_diff_view = load_terrain_texture(
            "assets/textures/pine forest textures/grass_medium_01_diff.png",
            "Terrain Grass Diff",
        );
        let grass_norm_view = load_terrain_texture(
            "assets/textures/pine forest textures/grass_medium_01_nor_gl.png",
            "Terrain Grass Norm",
        );
        let grass_rough_view = load_terrain_texture(
            "assets/textures/pine forest textures/grass_medium_01_rough.png",
            "Terrain Grass Rough",
        );
        let rock_diff_view = load_terrain_texture(
            "assets/textures/pine forest textures/rock_moss_set_01_diff.png",
            "Terrain Rock Diff",
        );
        let rock_norm_view = load_terrain_texture(
            "assets/textures/pine forest textures/rock_moss_set_01_nor_gl.png",
            "Terrain Rock Norm",
        );
        let rock_rough_view = load_terrain_texture(
            "assets/textures/pine forest textures/rock_moss_set_01_rough.png",
            "Terrain Rock Rough",
        );

        let terrain_sampler = self.device.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            ..Default::default()
        });

        self.terrain_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Terrain Bind Group"),
            layout: &self.terrain_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&grass_diff_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&grass_norm_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&grass_rough_view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&rock_diff_view),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::TextureView(&rock_norm_view),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: wgpu::BindingResource::TextureView(&rock_rough_view),
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: wgpu::BindingResource::Sampler(&terrain_sampler),
                },
            ],
        });

        // 3. Water Plane (Replaced with River Mesh)
        println!("Adding water (river) mesh...");
        let water_mesh = self.create_river_mesh(water_mat);
        self.objects.push(SceneObject {
            mesh_index: water_mesh,
            position: Vec3::new(0.0, 0.0, 0.0),
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
            model_bind_group: self
                .create_model_bind_group(Mat4::from_translation(Vec3::new(0.0, 0.0, 0.0))),
        });
        println!("Water mesh created with index: {}", water_mesh);

        // 4. Tree Variations
        println!("Placing trees...");
        let tree_models = [
            "assets/models/tree_pineDefaultA.glb",
            "assets/models/tree_pineTallA.glb",
            "assets/models/tree_pineRoundA.glb",
        ];

        // Helper for noise
        let noise = |x: f32, z: f32| -> f32 {
            (x * 0.05).sin() * (z * 0.05).cos() + (x * 0.15).sin() * 0.5 + (z * 0.08).cos() * 0.5
        };

        let mut tree_count = 0;
        for x in (-60..60).step_by(5) {
            for z in (-60..60).step_by(5) {
                let fx = x as f32;
                let fz = z as f32;

                let dist = (fx * fx + fz * fz).sqrt();
                if dist > 48.0 {
                    continue;
                } // Island edge

                // Calculate height and normal
                let height = self.calculate_terrain_height(fx, fz);

                let epsilon = 0.5;
                let hx1 = self.calculate_terrain_height(fx + epsilon, fz);
                let hz1 = self.calculate_terrain_height(fx, fz + epsilon);
                let v1 = Vec3::new(epsilon, hx1 - height, 0.0);
                let v2 = Vec3::new(0.0, hz1 - height, epsilon);
                let normal = v2.cross(v1).normalize();

                // Filter out:
                // - River Bed: height is already carved. But to filter OUT river bed, we need to know if we are IN it.
                // The river path is:
                let river_path = (fx * 0.1).sin() * 20.0;
                let dist_river = (fz - river_path).abs();
                if dist_river < 6.0 {
                    continue;
                } // River bed (5.0 + margin)

                if dist < 15.0 {
                    continue;
                } // Town plateau
                if normal.dot(Vec3::Y) < 0.707 {
                    continue;
                } // Too steep (>45 degrees)

                let density = noise(fx, fz);
                if density > 0.1 {
                    // Increased density threshold
                    // Random tree selection
                    let tree_idx = ((fx + fz).abs() as usize) % tree_models.len();
                    let tree_path = tree_models[tree_idx];

                    // Random position jitter
                    let jitter_x = (fx * 12.9898).sin() * 2.0;
                    let jitter_z = (fz * 78.233).cos() * 2.0;

                    // Use calculated height from helper function (recalc for jittered pos)
                    let pos_x = fx + jitter_x;
                    let pos_z = fz + jitter_z;
                    let pos_y = self.calculate_terrain_height(pos_x, pos_z);
                    let pos = Vec3::new(pos_x, pos_y, pos_z);

                    // Random rotation and scale
                    let rot_y = (fx * fz * 0.1).sin() * std::f32::consts::TAU;
                    let scale = (0.8 + ((fx + fz) * 0.1).sin().abs() * 0.4) * 2.0; // x2.0 scale

                    if let Ok(indices) = self.load_gltf(tree_path, self.pine_bark_mat) {
                        for idx in indices {
                            self.objects.push(SceneObject {
                                mesh_index: idx,
                                position: pos,
                                rotation: Quat::from_rotation_y(rot_y),
                                scale: Vec3::splat(scale),
                                model_bind_group: self.create_model_bind_group(
                                    Mat4::from_scale_rotation_translation(
                                        Vec3::splat(scale),
                                        Quat::from_rotation_y(rot_y),
                                        pos,
                                    ),
                                ),
                            });
                        }
                        tree_count += 1;
                    }
                }
            }
        }
        println!("Placed {} tree models.", tree_count);

        // 6. Tents and Campfire
        println!("Placing tents and campfire...");
        // Tent
        let tent_pos = Vec3::new(20.0, self.calculate_terrain_height(20.0, 20.0) + 0.15, 20.0);
        if let Ok(indices) = self.load_gltf("assets/models/tent_smallOpen.glb", self.tower_wood_mat)
        {
            for idx in indices {
                self.objects.push(SceneObject {
                    mesh_index: idx,
                    position: tent_pos,
                    rotation: Quat::from_rotation_y(0.7),
                    scale: Vec3::splat(2.0),
                    model_bind_group: self.create_model_bind_group(
                        Mat4::from_scale_rotation_translation(
                            Vec3::splat(2.0),
                            Quat::from_rotation_y(0.7),
                            tent_pos,
                        ),
                    ),
                });
            }
            println!(
                "Tent placed at ({}, {}, {}).",
                tent_pos.x, tent_pos.y, tent_pos.z
            );
        }
        // Campfire
        let camp_pos = Vec3::new(23.0, self.calculate_terrain_height(23.0, 18.0) - 0.05, 18.0);
        if let Ok(indices) = self.load_gltf("assets/models/campfire_logs.glb", self.tower_wood_mat)
        {
            for idx in indices {
                self.objects.push(SceneObject {
                    mesh_index: idx,
                    position: camp_pos,
                    rotation: Quat::IDENTITY,
                    scale: Vec3::splat(1.5),
                    model_bind_group: self.create_model_bind_group(
                        Mat4::from_scale_rotation_translation(
                            Vec3::splat(1.5),
                            Quat::IDENTITY,
                            camp_pos,
                        ),
                    ),
                });
            }
            println!(
                "Campfire placed at ({}, {}, {}).",
                camp_pos.x, camp_pos.y, camp_pos.z
            );
        }

        // Structure
        println!("Placing structure...");
        let peak_h = self.calculate_terrain_height(10.0, 10.0); // Offset from center peak
        let struct_pos = Vec3::new(10.0, peak_h, 10.0);
        if let Ok(indices) = self.load_gltf(
            "assets/models/tent_detailedClosed.glb",
            self.tower_stone_mat,
        ) {
            for idx in indices {
                self.objects.push(SceneObject {
                    mesh_index: idx,
                    position: struct_pos,
                    rotation: Quat::IDENTITY,
                    scale: Vec3::splat(3.0),
                    model_bind_group: self.create_model_bind_group(
                        Mat4::from_scale_rotation_translation(
                            Vec3::splat(3.0),
                            Quat::IDENTITY,
                            struct_pos,
                        ),
                    ),
                });
            }
            println!("Structure placed.");
        }

        // 7. Skybox
        println!("Creating skybox...");
        let sky_mat = 0; // Placeholder material index (not used in skybox rendering)
        self.sky_mesh_index = self.create_sphere_mesh(1000.0, 64, 32, sky_mat); // 1000.0 size

        println!(
            "Floating island scene complete. Objects: {}",
            self.objects.len()
        );
    }

    fn create_floating_island_terrain(
        &mut self,
        radius: f32,
        height: f32,
        subdivisions: u32,
    ) -> usize {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        // FBM height calculation helper
        let get_height_fbm = |x: f32, z: f32| -> f32 {
            let mut height = 0.0;
            // Layer 1: Hills
            height += (x * 0.05).sin() * (z * 0.05).cos() * 6.0;
            // Layer 2: Details
            height += (x * 0.2).sin() * (z * 0.15).cos() * 2.0;
            // Layer 3: Mountain Peak (z < -10.0)
            if z < -10.0 {
                let mountain_factor = ((-z - 10.0) / 20.0).clamp(0.0, 1.0);
                height += mountain_factor * 25.0;
            }
            // River Bed carving
            let river_path = (x * 0.1).sin() * 20.0; // Meandering X
            let dist_river = (z - river_path).abs();
            if dist_river < 5.0 {
                height -= (5.0 - dist_river) * 2.0;
            }
            // Town Plateau flattening
            let dist_center = (x * x + z * z).sqrt();
            if dist_center < 15.0 {
                // Blend to flat 5.0
                let blend = (dist_center / 15.0).powf(2.0);
                height = height * blend + 5.0 * (1.0 - blend);
            }
            height
        };

        // Cone-based strategy:
        // - Top Cap: Use FBM height logic
        // - Bottom Cone: Vertices taper from radius at Y=0 to radius 0 at Y=-height
        // - Recalculate normals after noise

        let rings = subdivisions;
        let segments = subdivisions * 2; // More angular resolution

        // Generate Top Cap
        // Center vertex
        let center_idx = vertices.len() as u32;
        let center_height = get_height_fbm(0.0, 0.0);
        vertices.push(Vertex {
            position: [0.0, center_height, 0.0],
            normal: [0.0, 1.0, 0.0],
            uv: [0.5, 0.5],
            color: [1.0, 1.0, 1.0, 1.0],
            tangent: [1.0, 0.0, 0.0, 1.0],
        });

        // Top cap rings
        for r in 1..=rings {
            let r_frac = r as f32 / rings as f32;
            let ring_radius = radius * r_frac;

            for s in 0..segments {
                let theta = (s as f32 / segments as f32) * 2.0 * std::f32::consts::PI;
                let x = ring_radius * theta.cos();
                let z = ring_radius * theta.sin();

                // Apply FBM height
                let final_y = get_height_fbm(x, z);

                vertices.push(Vertex {
                    position: [x, final_y, z],
                    normal: [0.0, 1.0, 0.0], // Will recalculate later
                    uv: [x / radius, z / radius],
                    color: [1.0, 1.0, 1.0, 1.0],
                    tangent: [1.0, 0.0, 0.0, 1.0],
                });
            }
        }

        // Top cap indices
        // Connect center to first ring
        for s in 0..segments {
            let next_s = (s + 1) % segments;
            indices.extend_from_slice(&[center_idx, center_idx + 1 + s, center_idx + 1 + next_s]);
        }

        // Connect rings
        for r in 0..(rings - 1) {
            let base_inner = center_idx + 1 + r * segments;
            let base_outer = center_idx + 1 + (r + 1) * segments;

            for s in 0..segments {
                let next_s = (s + 1) % segments;

                indices.extend_from_slice(&[base_inner + s, base_outer + next_s, base_outer + s]);
                indices.extend_from_slice(&[
                    base_inner + s,
                    base_inner + next_s,
                    base_outer + next_s,
                ]);
            }
        }

        // Generate Bottom Cone (tapers from radius to 0 at Y=-height)
        let cone_rings = subdivisions / 2; // Fewer rings on cone for performance
        let bottom_vertex_start = vertices.len();

        for r in 0..=cone_rings {
            let r_frac = r as f32 / cone_rings as f32;
            let ring_radius = radius * (1.0 - r_frac); // Taper to 0
            let ring_y = -height * r_frac;

            for s in 0..segments {
                let theta = (s as f32 / segments as f32) * 2.0 * std::f32::consts::PI;
                let x = ring_radius * theta.cos();
                let z = ring_radius * theta.sin();

                // Apply tapered FBM noise to match top surface and ensure smooth transition
                let base_fbm_height = get_height_fbm(x, z);
                let noise_contribution = base_fbm_height * (1.0 - r_frac); // Taper to 0 at tip
                let final_y = ring_y + noise_contribution;

                vertices.push(Vertex {
                    position: [x, final_y, z],
                    normal: [0.0, -1.0, 0.0], // Will recalculate later
                    uv: [x / radius, z / radius],
                    color: [1.0, 1.0, 1.0, 1.0],
                    tangent: [1.0, 0.0, 0.0, 1.0],
                });
            }
        }

        // Bottom cone indices (reversed winding for downward facing)
        for r in 0..cone_rings {
            let base_upper = (bottom_vertex_start + (r as usize) * (segments as usize)) as u32;
            let base_lower =
                (bottom_vertex_start + ((r + 1) as usize) * (segments as usize)) as u32;

            for s in 0..segments {
                let next_s = (s + 1) % segments;

                if r == cone_rings - 1 && segments > 0 {
                    // Last ring connects to tip (single point)
                    indices.extend_from_slice(&[
                        base_upper + s,
                        base_lower, // Tip (first vertex of last ring)
                        base_upper + next_s,
                    ]);
                } else {
                    // Normal quad split
                    indices.extend_from_slice(&[
                        base_upper + s,
                        base_upper + next_s,
                        base_lower + next_s,
                    ]);
                    indices.extend_from_slice(&[
                        base_upper + s,
                        base_lower + next_s,
                        base_lower + s,
                    ]);
                }
            }
        }

        // Recalculate normals
        let mut new_normals = vec![Vec3::ZERO; vertices.len()];

        for tri in indices.chunks(3) {
            let i0 = tri[0] as usize;
            let i1 = tri[1] as usize;
            let i2 = tri[2] as usize;

            let p0 = Vec3::from(vertices[i0].position);
            let p1 = Vec3::from(vertices[i1].position);
            let p2 = Vec3::from(vertices[i2].position);

            let edge1 = p1 - p0;
            let edge2 = p2 - p0;
            let face_normal = edge1.cross(edge2).normalize();

            new_normals[i0] += face_normal;
            new_normals[i1] += face_normal;
            new_normals[i2] += face_normal;
        }

        for (i, vertex) in vertices.iter_mut().enumerate() {
            let len_sq = new_normals[i].length_squared();
            let normal = if len_sq > 0.0001 {
                new_normals[i].normalize()
            } else {
                Vec3::Y // Default up
            };
            vertex.normal = normal.to_array();
        }

        // Create unified mesh (material index 0, not used in terrain pipeline)
        self.create_mesh_from_data(&vertices, &indices, 0)
    }

    fn create_model_bind_group(&self, model_mat: Mat4) -> wgpu::BindGroup {
        let buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Model Buffer"),
                contents: bytemuck::cast_slice(&[ModelUniforms {
                    model: model_mat.to_cols_array_2d(),
                }]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Model Bind Group"),
            layout: &self.model_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
        })
    }

    fn create_material_from_color(&mut self, name: &str, color: [f32; 4]) -> usize {
        let size = wgpu::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        };
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some(name),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        let data = [
            (color[0] * 255.0) as u8,
            (color[1] * 255.0) as u8,
            (color[2] * 255.0) as u8,
            (color[3] * 255.0) as u8,
        ];

        self.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4),
                rows_per_image: Some(1),
            },
            size,
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = self.device.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(name),
            layout: &self.material_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        self.materials.push(Material {
            name: name.to_string(),
            bind_group,
        });
        self.materials.len() - 1
    }

    fn create_material_from_texture(&mut self, name: &str, path: &str) -> usize {
        let img = image::open(path)
            .unwrap_or_else(|_| panic!("Missing texture: {}", path))
            .to_rgba8();
        let size = wgpu::Extent3d {
            width: img.width(),
            height: img.height(),
            depth_or_array_layers: 1,
        };
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some(name),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        self.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &img,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * img.width()),
                rows_per_image: Some(img.height()),
            },
            size,
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = self.device.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            ..Default::default()
        });

        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(name),
            layout: &self.material_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        self.materials.push(Material {
            name: name.to_string(),
            bind_group,
        });
        self.materials.len() - 1
    }

    fn create_sphere_mesh(
        &mut self,
        radius: f32,
        sectors: u32,
        stacks: u32,
        mat_idx: usize,
    ) -> usize {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        for i in 0..=stacks {
            let v = i as f32 / stacks as f32;
            let phi = v * std::f32::consts::PI;

            for j in 0..=sectors {
                let u = j as f32 / sectors as f32;
                let theta = u * 2.0 * std::f32::consts::PI;

                let x = radius * phi.sin() * theta.cos();
                let y = radius * phi.cos();
                let z = radius * phi.sin() * theta.sin();

                vertices.push(Vertex {
                    position: [x, y, z],
                    normal: [x / radius, y / radius, z / radius],
                    uv: [u, v],
                    color: [1.0, 1.0, 1.0, 1.0],
                    tangent: [0.0; 4],
                });
            }
        }

        for i in 0..stacks {
            for j in 0..sectors {
                let first = (i * (sectors + 1)) + j;
                let second = first + sectors + 1;
                indices.extend_from_slice(&[
                    first,
                    second,
                    first + 1,
                    second,
                    second + 1,
                    first + 1,
                ]);
            }
        }

        self.create_mesh_from_data(&vertices, &indices, mat_idx)
    }

    fn calculate_terrain_height(&self, x: f32, z: f32) -> f32 {
        let mut height = (x * 0.05).sin() * (z * 0.05).cos() * 6.0;
        height += (x * 0.2).sin() * (z * 0.15).cos() * 2.0;
        if z < -10.0 {
            let mountain_factor = ((-z - 10.0) / 20.0).clamp(0.0, 1.0);
            height += mountain_factor * 25.0;
        }
        // River Bed carving - flatten terrain along river path
        let river_path = (x * 0.1).sin() * 20.0;
        let dist_river = (z - river_path).abs();
        let river_mask = (dist_river / 15.0).clamp(0.0, 1.0);
        height *= river_mask;
        if dist_river < 5.0 {
            height -= (5.0 - dist_river) * 2.0;
        }
        // Town Plateau logic
        let dist_center = (x * x + z * z).sqrt();
        if dist_center < 15.0 {
            let blend = (dist_center / 15.0).powf(2.0);
            height = height * blend + 5.0 * (1.0 - blend);
        }
        height
    }

    fn create_river_mesh(&mut self, mat_idx: usize) -> usize {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        let steps = 100;
        let min_x = -48.0;
        let max_x = 48.0;
        let step_size = (max_x - min_x) / steps as f32;

        for i in 0..=steps {
            let x = min_x + i as f32 * step_size;
            let z_center = (x * 0.1).sin() * 20.0;

            // Water level: above terrain
            let water_level = -2.0; // Lowered significantly to sit within the riverbed

            let half_width = 10.0;

            // Left
            vertices.push(Vertex {
                position: [x, water_level, z_center - half_width],
                normal: [0.0, 1.0, 0.0],
                uv: [x / 50.0, (z_center - half_width) / 50.0],
                color: [1.0; 4],
                tangent: [1.0, 0.0, 0.0, 1.0],
            });

            // Right
            vertices.push(Vertex {
                position: [x, water_level, z_center + half_width],
                normal: [0.0, 1.0, 0.0],
                uv: [x / 50.0, (z_center + half_width) / 50.0],
                color: [1.0; 4],
                tangent: [1.0, 0.0, 0.0, 1.0],
            });
        }

        for i in 0..steps {
            let base = i * 2;
            // Tri 1: 0, 1, 2
            indices.extend_from_slice(&[base, base + 1, base + 2]);
            // Tri 2: 1, 3, 2
            indices.extend_from_slice(&[base + 1, base + 3, base + 2]);
        }

        self.create_mesh_from_data(&vertices, &indices, mat_idx)
    }

    #[allow(dead_code)] // Utility method kept for future terrain demo use
    fn create_plane_mesh(&mut self, size: f32, mat_idx: usize) -> usize {
        let s = size / 2.0;
        let uv_scale = 50.0; // Increase tiling for better detail
        let vertices = vec![
            Vertex {
                position: [-s, 0.0, -s],
                normal: [0.0, 1.0, 0.0],
                uv: [0.0, 0.0],
                color: [1.0; 4],
                tangent: [1.0, 0.0, 0.0, 1.0],
            },
            Vertex {
                position: [s, 0.0, -s],
                normal: [0.0, 1.0, 0.0],
                uv: [uv_scale, 0.0],
                color: [1.0; 4],
                tangent: [1.0, 0.0, 0.0, 1.0],
            },
            Vertex {
                position: [s, 0.0, s],
                normal: [0.0, 1.0, 0.0],
                uv: [uv_scale, uv_scale],
                color: [1.0; 4],
                tangent: [1.0, 0.0, 0.0, 1.0],
            },
            Vertex {
                position: [-s, 0.0, s],
                normal: [0.0, 1.0, 0.0],
                uv: [0.0, uv_scale],
                color: [1.0; 4],
                tangent: [1.0, 0.0, 0.0, 1.0],
            },
        ];
        // Flip indices to correct winding order (CCW for Up-facing normal)
        let indices = vec![0, 2, 1, 0, 3, 2];
        self.create_mesh_from_data(&vertices, &indices, mat_idx)
    }

    #[allow(dead_code)]
    fn create_cube_mesh(&mut self, size: f32, mat_idx: usize) -> usize {
        let s = size / 2.0;
        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        let mut add_face = |p: [Vec3; 4], n: Vec3| {
            let base = vertices.len() as u32;
            vertices.push(Vertex {
                position: p[0].to_array(),
                normal: n.to_array(),
                uv: [0.0, 1.0],
                color: [1.0; 4],
                tangent: [0.0; 4],
            });
            vertices.push(Vertex {
                position: p[1].to_array(),
                normal: n.to_array(),
                uv: [1.0, 1.0],
                color: [1.0; 4],
                tangent: [0.0; 4],
            });
            vertices.push(Vertex {
                position: p[2].to_array(),
                normal: n.to_array(),
                uv: [1.0, 0.0],
                color: [1.0; 4],
                tangent: [0.0; 4],
            });
            vertices.push(Vertex {
                position: p[3].to_array(),
                normal: n.to_array(),
                uv: [0.0, 0.0],
                color: [1.0; 4],
                tangent: [0.0; 4],
            });
            indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
        };

        add_face(
            [
                Vec3::new(-s, -s, s),
                Vec3::new(s, -s, s),
                Vec3::new(s, s, s),
                Vec3::new(-s, s, s),
            ],
            Vec3::Z,
        );
        add_face(
            [
                Vec3::new(s, -s, -s),
                Vec3::new(-s, -s, -s),
                Vec3::new(-s, s, -s),
                Vec3::new(s, s, -s),
            ],
            -Vec3::Z,
        );
        add_face(
            [
                Vec3::new(s, -s, s),
                Vec3::new(s, -s, -s),
                Vec3::new(s, s, -s),
                Vec3::new(s, s, s),
            ],
            Vec3::X,
        );
        add_face(
            [
                Vec3::new(-s, -s, -s),
                Vec3::new(-s, -s, s),
                Vec3::new(-s, s, s),
                Vec3::new(-s, s, -s),
            ],
            -Vec3::X,
        );
        add_face(
            [
                Vec3::new(-s, s, s),
                Vec3::new(s, s, s),
                Vec3::new(s, s, -s),
                Vec3::new(-s, s, -s),
            ],
            Vec3::Y,
        );
        add_face(
            [
                Vec3::new(-s, -s, -s),
                Vec3::new(s, -s, -s),
                Vec3::new(s, -s, s),
                Vec3::new(-s, -s, s),
            ],
            -Vec3::Y,
        );

        self.create_mesh_from_data(&vertices, &indices, mat_idx)
    }

    fn create_mesh_from_data(
        &mut self,
        vertices: &[Vertex],
        indices: &[u32],
        mat_idx: usize,
    ) -> usize {
        let vertex_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Vertex Buffer"),
                contents: bytemuck::cast_slice(vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });
        let index_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Index Buffer"),
                contents: bytemuck::cast_slice(indices),
                usage: wgpu::BufferUsages::INDEX,
            });

        self.meshes.push(Mesh {
            vertex_buffer,
            index_buffer,
            num_indices: indices.len() as u32,
            material_index: mat_idx,
        });
        self.meshes.len() - 1
    }

    fn load_gltf(
        &mut self,
        path: &str,
        default_mat: usize,
    ) -> Result<Vec<usize>, Box<dyn std::error::Error>> {
        let loaded = gltf_loader::load_gltf(std::path::Path::new(path))?;
        let mut indices = Vec::new();
        for mesh_data in loaded {
            let vertices: Vec<Vertex> = mesh_data
                .vertices
                .iter()
                .map(|v| Vertex {
                    position: v.position,
                    normal: v.normal,
                    uv: v.uv,
                    color: v.color,
                    tangent: v.tangent,
                })
                .collect();

            // Material name-based selection - case-insensitive matching
            let mat_name = mesh_data
                .material_name
                .as_deref()
                .unwrap_or("default")
                .to_lowercase();
            let selected_mat = if mat_name.contains("bark")
                || mat_name.contains("trunk")
                || mat_name.contains("brown")
                || mat_name.contains("woodbark")
            {
                println!(
                    "  DEBUG: Material '{}' matched BARK pattern -> index {}",
                    mat_name, self.pine_bark_mat
                );
                self.pine_bark_mat
            } else if mat_name.contains("leaf")
                || mat_name.contains("twig")
                || mat_name.contains("green")
                || mat_name.contains("foliage")
                || mat_name.contains("pine")
                || mat_name.contains("needle")
            {
                println!(
                    "  DEBUG: Material '{}' matched LEAF pattern -> index {}",
                    mat_name, self.pine_leaves_mat
                );
                self.pine_leaves_mat
            } else if mat_name.contains("wood")
                && !mat_name.contains("woodbark")
                && !mat_name.contains("bark")
            {
                println!(
                    "  DEBUG: Material '{}' matched WOOD pattern -> index {}",
                    mat_name, self.tower_wood_mat
                );
                self.tower_wood_mat
            } else if mat_name.contains("stone")
                || mat_name.contains("wall")
                || mat_name.contains("rock")
                || mat_name.contains("stones")
            {
                println!(
                    "  DEBUG: Material '{}' matched STONE pattern -> index {}",
                    mat_name, self.tower_stone_mat
                );
                self.tower_stone_mat
            } else if mat_name.contains("brick") {
                println!(
                    "  DEBUG: Material '{}' matched BRICK pattern -> index {}",
                    mat_name, self.tower_stone_mat
                );
                self.tower_stone_mat
            } else if mat_name.contains("color") {
                if mat_name.contains("red") {
                    println!(
                        "  DEBUG: Material '{}' matched COLOR-RED pattern -> index {}",
                        mat_name, self.tower_wood_mat
                    );
                    self.tower_wood_mat
                } else {
                    println!(
                        "  DEBUG: Material '{}' matched COLOR (other) pattern -> index {}",
                        mat_name, default_mat
                    );
                    default_mat
                }
            } else {
                println!(
                    "  DEBUG: Material '{}' NO MATCH, using default -> index {}",
                    mat_name, default_mat
                );
                default_mat
            };

            println!("DEBUG MATCH: {} -> {} -> {}", path, mat_name, selected_mat);

            indices.push(self.create_mesh_from_data(&vertices, &mesh_data.indices, selected_mat));
        }
        Ok(indices)
    }

    fn update(&mut self) {
        let view = Mat4::look_at_rh(
            self.camera_pos,
            self.camera_pos
                + Quat::from_rotation_y(self.camera_yaw)
                    * Quat::from_rotation_x(self.camera_pitch)
                    * Vec3::NEG_Z, // Use NEG_Z
            Vec3::Y,
        );
        // println!("Camera Pos: {:?}, Yaw: {}, Pitch: {}", self.camera_pos, self.camera_yaw, self.camera_pitch);
        let proj = Mat4::perspective_rh(
            45.0_f32.to_radians(),
            self.config.width as f32 / self.config.height as f32,
            0.1,
            2000.0,
        ); // Z-far: 2000.0

        self.queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&[CameraUniforms {
                view_proj: (proj * view).to_cols_array_2d(),
                camera_pos: self.camera_pos.to_array(),
                _padding: 0.0,
            }]),
        );
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        // Update light view_proj for shadow mapping
        let light_pos = Vec3::new(100.0, 50.0, 50.0);
        let light_view = Mat4::look_at_rh(light_pos, Vec3::ZERO, Vec3::Y);
        let light_proj = Mat4::orthographic_rh(-100.0, 100.0, -100.0, 100.0, 0.1, 300.0);
        let light_view_proj = light_proj * light_view;

        // Update light uniforms with shadow view_proj
        self.queue.write_buffer(
            &self.light_buffer,
            0,
            bytemuck::cast_slice(&[LightUniforms {
                view_proj: light_view_proj.to_cols_array_2d(),
                position: light_pos.to_array(),
                _padding: 0.0,
                color: [1.2, 1.1, 1.0],
                _padding2: 0.0,
            }]),
        );

        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        // Shadow Pass
        {
            let mut shadow_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Shadow Pass"),
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.shadow_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            shadow_pass.set_pipeline(&self.shadow_pipeline);
            shadow_pass.set_bind_group(0, &self.shadow_bind_group, &[]);

            // Render objects
            for obj in &self.objects {
                let mesh = &self.meshes[obj.mesh_index];
                shadow_pass.set_bind_group(1, &obj.model_bind_group, &[]);
                shadow_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                shadow_pass
                    .set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                shadow_pass.draw_indexed(0..mesh.num_indices, 0, 0..1);
            }

            // Render terrain to cast shadows
            let terrain_mesh = &self.meshes[self.terrain_mesh_index];
            shadow_pass.set_bind_group(1, &self.terrain_model_bind_group, &[]);
            shadow_pass.set_vertex_buffer(0, terrain_mesh.vertex_buffer.slice(..));
            shadow_pass.set_index_buffer(
                terrain_mesh.index_buffer.slice(..),
                wgpu::IndexFormat::Uint32,
            );
            shadow_pass.draw_indexed(0..terrain_mesh.num_indices, 0, 0..1);
        }

        // Main Pass
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.msaa_texture,
                    resolve_target: Some(&view),
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.1,
                            b: 0.15,
                            a: 1.0,
                        }), // Dark Blue-Gray
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_texture,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            render_pass.set_pipeline(&self.sky_pipeline);
            render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
            render_pass.set_bind_group(1, &self.sky_bind_group, &[]);
            let sky_mesh = &self.meshes[self.sky_mesh_index];
            render_pass.set_vertex_buffer(0, sky_mesh.vertex_buffer.slice(..));
            render_pass
                .set_index_buffer(sky_mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            render_pass.draw_indexed(0..sky_mesh.num_indices, 0, 0..1);

            // Terrain rendering pass
            render_pass.set_pipeline(&self.terrain_pipeline);
            render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
            render_pass.set_bind_group(1, &self.light_bind_group, &[]);
            render_pass.set_bind_group(2, &self.terrain_bind_group, &[]);
            render_pass.set_bind_group(3, &self.terrain_model_bind_group, &[]);
            let terrain_mesh = &self.meshes[self.terrain_mesh_index];
            render_pass.set_vertex_buffer(0, terrain_mesh.vertex_buffer.slice(..));
            render_pass.set_index_buffer(
                terrain_mesh.index_buffer.slice(..),
                wgpu::IndexFormat::Uint32,
            );
            render_pass.draw_indexed(0..terrain_mesh.num_indices, 0, 0..1);

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
            render_pass.set_bind_group(1, &self.light_bind_group, &[]);

            for obj in &self.objects {
                let mesh = &self.meshes[obj.mesh_index];
                let material = &self.materials[mesh.material_index];
                render_pass.set_bind_group(2, &material.bind_group, &[]);
                render_pass.set_bind_group(3, &obj.model_bind_group, &[]);
                render_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                render_pass
                    .set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                render_pass.draw_indexed(0..mesh.num_indices, 0, 0..1);
            }
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
        Ok(())
    }

    fn handle_input(&mut self, event: &WindowEvent) -> bool {
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
                let speed = 0.5;
                if *state == ElementState::Pressed {
                    match key {
                        KeyCode::KeyW => {
                            self.camera_pos +=
                                Quat::from_rotation_y(self.camera_yaw) * Vec3::NEG_Z * speed
                        }
                        KeyCode::KeyS => {
                            self.camera_pos +=
                                Quat::from_rotation_y(self.camera_yaw) * Vec3::Z * speed
                        }
                        KeyCode::KeyA => {
                            self.camera_pos +=
                                Quat::from_rotation_y(self.camera_yaw) * Vec3::NEG_X * speed
                        }
                        KeyCode::KeyD => {
                            self.camera_pos +=
                                Quat::from_rotation_y(self.camera_yaw) * Vec3::X * speed
                        }
                        KeyCode::Space => self.camera_pos.y += speed,
                        KeyCode::ShiftLeft => self.camera_pos.y -= speed,
                        _ => {}
                    }
                }
                true
            }
            WindowEvent::MouseInput {
                state,
                button: MouseButton::Right,
                ..
            } => {
                self.mouse_pressed = *state == ElementState::Pressed;
                true
            }
            _ => false,
        }
    }

    fn handle_mouse_motion(&mut self, delta: (f64, f64)) {
        if self.mouse_pressed {
            let sensitivity = 0.005;
            self.camera_yaw += (delta.0 as f32) * sensitivity;
            self.camera_pitch += (delta.1 as f32) * sensitivity;
        }
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);

            self.depth_texture = self
                .device
                .create_texture(&wgpu::TextureDescriptor {
                    label: Some("Depth Texture"),
                    size: wgpu::Extent3d {
                        width: self.config.width,
                        height: self.config.height,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 4,
                    dimension: wgpu::TextureDimension::D2,
                    format: wgpu::TextureFormat::Depth32Float,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                    view_formats: &[],
                })
                .create_view(&wgpu::TextureViewDescriptor::default());

            self.msaa_texture = self
                .device
                .create_texture(&wgpu::TextureDescriptor {
                    label: Some("MSAA Texture"),
                    size: wgpu::Extent3d {
                        width: self.config.width,
                        height: self.config.height,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 4,
                    dimension: wgpu::TextureDimension::D2,
                    format: self.config.format,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                    view_formats: &[],
                })
                .create_view(&wgpu::TextureViewDescriptor::default());
        }
    }
}

struct App {
    state: Option<ShowcaseApp>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.state.is_none() {
            let window_attrs = Window::default_attributes().with_title("AstraWeave Showcase V2");
            let window = Arc::new(event_loop.create_window(window_attrs).unwrap());
            let state = pollster::block_on(ShowcaseApp::new(window));
            self.state = Some(state);
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        if let Some(state) = &mut self.state {
            if id == state.window.id() && !state.handle_input(&event) {
                match event {
                    WindowEvent::CloseRequested => event_loop.exit(),
                    WindowEvent::Resized(physical_size) => state.resize(physical_size),
                    WindowEvent::RedrawRequested => {
                        state.update();
                        match state.render() {
                            Ok(_) => {}
                            Err(wgpu::SurfaceError::Lost) => {
                                state.resize(winit::dpi::PhysicalSize::new(
                                    state.config.width,
                                    state.config.height,
                                ))
                            } // Reconfigure
                            Err(wgpu::SurfaceError::OutOfMemory) => event_loop.exit(),
                            Err(e) => eprintln!("{:?}", e),
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
        _device_id: DeviceId,
        event: DeviceEvent,
    ) {
        if let Some(state) = &mut self.state {
            if let DeviceEvent::MouseMotion { delta } = event {
                state.handle_mouse_motion(delta);
            }
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(state) = &mut self.state {
            state.window.request_redraw();
        }
    }
}

fn main() {
    env_logger::init();
    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);
    let mut app = App { state: None };
    event_loop.run_app(&mut app).unwrap();
}
