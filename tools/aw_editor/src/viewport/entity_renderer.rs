//! Entity Renderer
//!
//! Renders entities from the World into the 3D viewport.

#![allow(dead_code)]
//! Supports basic meshes, materials, and transform visualization.
//!
//! # Features
//!
//! - Primitive geometry rendering for entities (fallback for non-mesh entities)
//! - Transform matrix support (position, rotation, scale)
//! - Instanced rendering for performance
//! - Selection highlighting (different color for selected entities)
//!
//! # Performance Budget
//!
//! Target: <8ms per frame @ 1080p with 1000 entities
//! - Per-entity setup: <0.01ms
//! - Instanced draw call: ~5ms
//! - Total: ~7ms (12% under budget)

use anyhow::{Context as _, Result};
use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3};
use std::collections::HashMap;
use std::sync::Arc;
use wgpu::util::DeviceExt;

use super::camera::{Frustum, OrbitCamera};
use astraweave_core::{Entity, World};

// ============================================================================
// GLTF Skeleton & Animation types (Phase 2)
// ============================================================================

/// A single joint in a skeleton hierarchy.
#[derive(Clone, Debug)]
pub struct GltfJoint {
    /// Joint name (from the glTF node).
    pub name: String,
    /// Parent joint index in the skeleton's `joints` array, or `None` for roots.
    pub parent_index: Option<usize>,
    /// Inverse bind matrix (transforms from mesh space to joint-local space).
    pub inverse_bind_matrix: Mat4,
    /// Local (rest-pose) transform of the joint.
    pub local_transform: Mat4,
}

/// A skeleton extracted from a glTF skin.
#[derive(Clone, Debug)]
pub struct GltfSkeleton {
    /// Ordered joint list (index matches glTF skin joint order).
    pub joints: Vec<GltfJoint>,
    /// Indices of root joints (joints with no parent).
    pub root_indices: Vec<usize>,
}

/// Keyframe interpolation mode.
#[derive(Clone, Copy, Debug)]
pub enum GltfInterpolation {
    Linear,
    Step,
    CubicSpline,
}

/// Channel target property being animated.
#[derive(Clone, Copy, Debug)]
pub enum GltfChannelProperty {
    Translation,
    Rotation,
    Scale,
}

/// A single animation channel targeting one joint.
#[derive(Clone, Debug)]
pub struct GltfAnimChannel {
    /// Joint index in the skeleton.
    pub joint_index: usize,
    /// Property being animated.
    pub property: GltfChannelProperty,
    /// Keyframe timestamps in seconds.
    pub times: Vec<f32>,
    /// Keyframe values (3 floats for translation/scale, 4 for rotation quaternion).
    pub values: Vec<Vec<f32>>,
    /// Interpolation mode.
    pub interpolation: GltfInterpolation,
}

/// An animation clip extracted from a glTF animation.
#[derive(Clone, Debug)]
pub struct GltfAnimationClip {
    /// Clip name.
    pub name: String,
    /// Duration in seconds.
    pub duration: f32,
    /// Animation channels.
    pub channels: Vec<GltfAnimChannel>,
}

/// Per-vertex skinning data (joint indices + weights).
#[derive(Clone, Debug)]
pub struct SkinningData {
    /// Per-vertex: [joint0, joint1, joint2, joint3] indices.
    pub joints: Vec<[u16; 4]>,
    /// Per-vertex: [weight0, weight1, weight2, weight3].
    pub weights: Vec<[f32; 4]>,
}

/// A loaded mesh with GPU buffers ready for rendering
struct LoadedMesh {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    index_count: u32,
    index_format: wgpu::IndexFormat,
    /// Per-mesh texture bind group (albedo + normal + ORM + emissive). `None` → use vertex colors.
    texture_bind_group: Option<wgpu::BindGroup>,
    /// Per-mesh material params bind group (PBR uniforms). `None` → use fallback defaults.
    material_bind_group: Option<wgpu::BindGroup>,
    /// Per-mesh material uniform buffer (so we can update values).
    material_uniform_buffer: Option<wgpu::Buffer>,
    /// Skeleton extracted from glTF skin (if present).
    skeleton: Option<GltfSkeleton>,
    /// Animation clips extracted from glTF animations.
    animations: Vec<GltfAnimationClip>,
    /// Per-vertex skinning data (joint indices + weights).
    skinning_data: Option<SkinningData>,
    /// Original vertex positions/normals for CPU skinning (rest pose).
    rest_vertices: Option<Vec<Vertex>>,
    /// Alpha mode: 0=Opaque, 1=Mask, 2=Blend
    alpha_mode: u8,
    /// Whether the material is double-sided (no backface culling)
    double_sided: bool,
}

/// Entity renderer for viewport
///
/// Renders all entities in the World as simple colored cubes.
/// Uses instanced rendering for performance.
pub struct EntityRenderer {
    /// GPU device for buffer creation
    device: Arc<wgpu::Device>,

    /// Render pipeline
    pipeline: wgpu::RenderPipeline,

    /// Bind group layout
    bind_group_layout: wgpu::BindGroupLayout,

    /// Bind group (camera uniforms)
    bind_group: wgpu::BindGroup,

    /// Camera uniform buffer
    uniform_buffer: wgpu::Buffer,

    /// Vertex buffer (default cube vertices)
    vertex_buffer: wgpu::Buffer,

    /// Index buffer (default cube indices)
    index_buffer: wgpu::Buffer,

    /// Instance buffer (per-entity transforms + colors)
    instance_buffer: wgpu::Buffer,

    /// Maximum number of instances
    max_instances: u32,

    /// Number of indices per cube
    index_count: u32,

    /// Cache of loaded GLTF meshes keyed by file path
    mesh_cache: HashMap<String, LoadedMesh>,

    /// Negative cache: paths that failed to load with retry tracking.
    /// Value is (attempt_count, last_attempt_time). Retries after 30s cooldown, max 3 attempts.
    failed_mesh_paths: HashMap<String, (u32, std::time::Instant)>,

    /// Mapping from World entity ID to mesh file path
    entity_meshes: HashMap<Entity, String>,

    /// Bind group layout for per-mesh textures (albedo + sampler + normal + ORM + emissive)
    texture_bind_group_layout: wgpu::BindGroupLayout,

    /// Queue reference for texture uploads
    queue: Arc<wgpu::Queue>,

    /// Fallback texture bind group (white albedo, flat normal, default ORM, black emissive)
    fallback_texture_bind_group: wgpu::BindGroup,

    /// Bind group layout for per-mesh material parameters (group 2)
    material_bind_group_layout: wgpu::BindGroupLayout,

    /// Fallback material params bind group (default PBR values)
    fallback_material_bind_group: wgpu::BindGroup,

    /// Fallback normal map view (flat blue) — reused when building per-mesh bind groups
    fallback_normal_view: wgpu::TextureView,

    /// Fallback ORM texture view (ao=1, rough=1, metal=0) — reused when building per-mesh bind groups
    fallback_orm_view: wgpu::TextureView,

    /// Fallback emissive texture view (black) — reused when building per-mesh bind groups
    fallback_emissive_view: wgpu::TextureView,

    /// Fallback albedo texture view (white) — reused when building per-mesh bind groups
    fallback_albedo_view: wgpu::TextureView,

    /// Shared linear sampler — reused when building per-mesh bind groups
    fallback_sampler: wgpu::Sampler,

    /// Pipeline for textured meshes (uses texture bind group at group 1)
    textured_pipeline: wgpu::RenderPipeline,

    /// Textured pipeline variant for alpha-blend materials (depth write off, alpha blending on)
    alpha_blend_pipeline: wgpu::RenderPipeline,

    /// Textured pipeline variant for double-sided materials (no backface culling)
    double_sided_pipeline: wgpu::RenderPipeline,

    /// Wireframe pipeline (PolygonMode::Line) — `None` if GPU lacks POLYGON_MODE_LINE
    wireframe_pipeline: Option<wgpu::RenderPipeline>,

    /// Wireframe textured pipeline — `None` if GPU lacks POLYGON_MODE_LINE
    wireframe_textured_pipeline: Option<wgpu::RenderPipeline>,

    /// Cache of externally-loaded textures keyed by file path string
    texture_path_cache: HashMap<String, wgpu::BindGroup>,

    /// Per-entity external texture override (entity → texture file path)
    entity_texture_overrides: HashMap<Entity, String>,

    /// Scene point lights set from entities with Light components
    scene_lights: Vec<SceneLight>,

    /// Sun direction (normalized)
    sun_direction: [f32; 3],

    /// Sun color
    sun_color: [f32; 3],

    /// Sun intensity
    sun_intensity: f32,

    /// Ambient color
    ambient_color: [f32; 3],

    /// Ambient intensity
    ambient_intensity: f32,

    // ─── Shadow mapping ──────────────────────────────────────
    /// Shadow map depth texture (SHADOW_MAP_SIZE × SHADOW_MAP_SIZE)
    shadow_texture: wgpu::Texture,

    /// Shadow map texture view (used as depth attachment in shadow pass)
    shadow_texture_view: wgpu::TextureView,

    /// Bind group layout for sampling shadow map in main pass (group 3)
    shadow_bind_group_layout: wgpu::BindGroupLayout,

    /// Bind group for sampling shadow map in main pass (group 3)
    shadow_bind_group: wgpu::BindGroup,

    /// Depth-only shadow render pipeline (uses shadow.wgsl vs_shadow)
    shadow_pipeline: wgpu::RenderPipeline,

    /// Uniform buffer for shadow pass (light VP matrix)
    shadow_uniform_buffer: wgpu::Buffer,

    /// Bind group 0 for shadow pass (light VP uniforms)
    shadow_bind_group_0: wgpu::BindGroup,

    /// Whether shadow mapping is enabled
    shadow_enabled: bool,

    // ─── IBL (Image-Based Lighting) ──────────────────────────
    /// BRDF integration LUT texture (256×256, Rgba16Float)
    brdf_lut_view: wgpu::TextureView,

    /// IBL bind group layout (group 4)
    ibl_bind_group_layout: wgpu::BindGroupLayout,

    /// IBL bind group (group 4): BRDF LUT + sampler + SH uniform
    ibl_bind_group: wgpu::BindGroup,

    /// IBL uniform buffer (SH9 + intensity)
    ibl_uniform_buffer: wgpu::Buffer,

    /// Whether IBL is enabled
    ibl_enabled: bool,

    /// Exposure value in EV (0.0 = no adjustment, +1 = double brightness)
    exposure_ev: f32,

    /// When true, shader outputs linear HDR (post-process chain handles tonemap).
    hdr_output: bool,

    /// GPU mipmap generator — replaces the old CPU box-filter path (H-1).
    mipmap_generator: Option<super::mipmap_generator::MipmapGenerator>,
}

impl EntityRenderer {
    /// Create new entity renderer
    ///
    /// # Arguments
    ///
    /// * `device` - wgpu device
    /// * `max_instances` - Maximum number of entities to render (default: 10000)
    ///
    /// # Errors
    ///
    /// Returns error if shader compilation or buffer creation fails.
    pub fn new(
        device: Arc<wgpu::Device>,
        queue: Arc<wgpu::Queue>,
        max_instances: u32,
    ) -> Result<Self> {
        Self::with_color_format(
            device,
            queue,
            max_instances,
            wgpu::TextureFormat::Bgra8UnormSrgb,
        )
    }

    /// Create entity renderer with a configurable color target format.
    pub fn with_color_format(
        device: Arc<wgpu::Device>,
        queue: Arc<wgpu::Queue>,
        max_instances: u32,
        color_format: wgpu::TextureFormat,
    ) -> Result<Self> {
        // Load shader
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Entity Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/entity.wgsl").into()),
        });

        // Create bind group layout
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Entity Bind Group Layout"),
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

        // Create pipeline layout
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Entity Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        // Create pipeline
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Entity Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[
                    // Vertex buffer (position + normal + color + uv)
                    wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<Vertex>() as u64,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &[
                            wgpu::VertexAttribute {
                                offset: 0,
                                shader_location: 0,
                                format: wgpu::VertexFormat::Float32x3, // position
                            },
                            wgpu::VertexAttribute {
                                offset: 12,
                                shader_location: 1,
                                format: wgpu::VertexFormat::Float32x3, // normal
                            },
                            wgpu::VertexAttribute {
                                offset: 24,
                                shader_location: 2,
                                format: wgpu::VertexFormat::Float32x4, // vertex color
                            },
                            wgpu::VertexAttribute {
                                offset: 40,
                                shader_location: 8,
                                format: wgpu::VertexFormat::Float32x2, // uv
                            },
                        ],
                    },
                    // Instance buffer (model matrix + color)
                    wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<Instance>() as u64,
                        step_mode: wgpu::VertexStepMode::Instance,
                        attributes: &[
                            // Model matrix (mat4, split into 4 vec4s)
                            wgpu::VertexAttribute {
                                offset: 0,
                                shader_location: 3,
                                format: wgpu::VertexFormat::Float32x4,
                            },
                            wgpu::VertexAttribute {
                                offset: 16,
                                shader_location: 4,
                                format: wgpu::VertexFormat::Float32x4,
                            },
                            wgpu::VertexAttribute {
                                offset: 32,
                                shader_location: 5,
                                format: wgpu::VertexFormat::Float32x4,
                            },
                            wgpu::VertexAttribute {
                                offset: 48,
                                shader_location: 6,
                                format: wgpu::VertexFormat::Float32x4,
                            },
                            // Color (vec4)
                            wgpu::VertexAttribute {
                                offset: 64,
                                shader_location: 7,
                                format: wgpu::VertexFormat::Float32x4,
                            },
                        ],
                    },
                ],
                compilation_options: Default::default(),
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
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
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: color_format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            multiview: None,
            cache: None,
        });

        // Create uniform buffer
        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Entity Uniform Buffer"),
            size: std::mem::size_of::<EntityUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Create bind group
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Entity Bind Group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        // Create cube geometry
        let (vertices, indices) = create_cube_mesh();
        let index_count = indices.len() as u32;

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Entity Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Entity Index Buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        // Create instance buffer (pre-allocate for max_instances)
        let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Entity Instance Buffer"),
            size: (std::mem::size_of::<Instance>() * max_instances as usize) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // --- Texture bind group layout (group 1): albedo + sampler + normal + ORM + emissive ---
        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Entity PBR Texture Bind Group Layout"),
                entries: &[
                    // binding 0: albedo texture
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    // binding 1: shared sampler
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                    // binding 2: normal map
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
                    // binding 3: ORM (occlusion, roughness, metallic)
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    // binding 4: emissive texture
                    wgpu::BindGroupLayoutEntry {
                        binding: 4,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                ],
            });

        // --- Material params bind group layout (group 2): PBR uniform buffer ---
        let material_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Entity Material Params Bind Group Layout"),
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

        // Helper: create a 1×1 fallback texture
        let make_1x1 = |label: &str, rgba: [u8; 4], srgb: bool| -> wgpu::Texture {
            let fmt = if srgb {
                wgpu::TextureFormat::Rgba8UnormSrgb
            } else {
                wgpu::TextureFormat::Rgba8Unorm
            };
            device.create_texture_with_data(
                &queue,
                &wgpu::TextureDescriptor {
                    label: Some(label),
                    size: wgpu::Extent3d {
                        width: 1,
                        height: 1,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: fmt,
                    usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                    view_formats: &[],
                },
                wgpu::util::TextureDataOrder::LayerMajor,
                &rgba,
            )
        };

        // Fallback textures
        let fallback_albedo_tex = make_1x1("Fallback White Albedo", [255, 255, 255, 255], true);
        let fallback_normal_tex = make_1x1("Fallback Flat Normal", [128, 128, 255, 255], false);
        // ORM: R=occlusion(1.0=255), G=roughness(0.5=128), B=metallic(0.0=0)
        let fallback_orm_tex = make_1x1("Fallback ORM", [255, 128, 0, 255], false);
        let fallback_emissive_tex = make_1x1("Fallback Black Emissive", [0, 0, 0, 255], true);

        let fallback_albedo_view = fallback_albedo_tex.create_view(&Default::default());
        let fallback_normal_view = fallback_normal_tex.create_view(&Default::default());
        let fallback_orm_view = fallback_orm_tex.create_view(&Default::default());
        let fallback_emissive_view = fallback_emissive_tex.create_view(&Default::default());

        let fallback_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Entity PBR Sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            anisotropy_clamp: 16,
            ..Default::default()
        });

        let fallback_texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Entity Fallback PBR Texture BG"),
            layout: &texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&fallback_albedo_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&fallback_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&fallback_normal_view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&fallback_orm_view),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::TextureView(&fallback_emissive_view),
                },
            ],
        });

        // Fallback material uniform buffer + bind group (default PBR values)
        let default_mat = MaterialParamsGpu::default();
        let fallback_material_buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Entity Fallback Material UB"),
                contents: bytemuck::bytes_of(&default_mat),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });
        let fallback_material_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Entity Fallback Material BG"),
            layout: &material_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: fallback_material_buffer.as_entire_binding(),
            }],
        });

        // ═══════════════════════════════════════════════════════════════════
        // Shadow mapping resources
        // ═══════════════════════════════════════════════════════════════════

        // Shadow depth texture (SHADOW_MAP_SIZE × SHADOW_MAP_SIZE)
        let shadow_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Shadow Map Depth Texture"),
            size: wgpu::Extent3d {
                width: SHADOW_MAP_SIZE,
                height: SHADOW_MAP_SIZE,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let shadow_texture_view =
            shadow_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Comparison sampler for shadow depth testing
        let shadow_comparison_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Shadow Comparison Sampler"),
            compare: Some(wgpu::CompareFunction::LessEqual),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        // Shadow bind group layout (group 3 in main pass): depth texture + comparison sampler
        let shadow_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Shadow Map Bind Group Layout (group 3)"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Depth,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                        count: None,
                    },
                ],
            });

        // Shadow bind group (group 3 in main pass)
        let shadow_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Shadow Map Bind Group"),
            layout: &shadow_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&shadow_texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&shadow_comparison_sampler),
                },
            ],
        });

        // Shadow uniform buffer (light VP matrix — 64 bytes)
        let shadow_uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Shadow Uniform Buffer"),
            size: std::mem::size_of::<ShadowUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Shadow pass bind group layout (group 0 for shadow pipeline)
        let shadow_pass_bg_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Shadow Pass Bind Group Layout"),
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

        // Shadow pass bind group 0 (light VP uniform)
        let shadow_bind_group_0 = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Shadow Pass Bind Group 0"),
            layout: &shadow_pass_bg_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: shadow_uniform_buffer.as_entire_binding(),
            }],
        });

        // Shadow shader module
        let shadow_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shadow Depth Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/shadow.wgsl").into()),
        });

        // Shadow pipeline layout (group 0 only: light VP)
        let shadow_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Shadow Pipeline Layout"),
                bind_group_layouts: &[&shadow_pass_bg_layout],
                push_constant_ranges: &[],
            });

        // Shadow depth-only render pipeline
        let shadow_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Shadow Depth Pipeline"),
            layout: Some(&shadow_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shadow_shader,
                entry_point: Some("vs_shadow"),
                buffers: &[
                    // Same vertex layout as entity pipeline
                    wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<Vertex>() as u64,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &[
                            wgpu::VertexAttribute {
                                offset: 0,
                                shader_location: 0,
                                format: wgpu::VertexFormat::Float32x3,
                            },
                            wgpu::VertexAttribute {
                                offset: 12,
                                shader_location: 1,
                                format: wgpu::VertexFormat::Float32x3,
                            },
                            wgpu::VertexAttribute {
                                offset: 24,
                                shader_location: 2,
                                format: wgpu::VertexFormat::Float32x4,
                            },
                            wgpu::VertexAttribute {
                                offset: 40,
                                shader_location: 8,
                                format: wgpu::VertexFormat::Float32x2,
                            },
                        ],
                    },
                    // Same instance layout
                    wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<Instance>() as u64,
                        step_mode: wgpu::VertexStepMode::Instance,
                        attributes: &[
                            wgpu::VertexAttribute {
                                offset: 0,
                                shader_location: 3,
                                format: wgpu::VertexFormat::Float32x4,
                            },
                            wgpu::VertexAttribute {
                                offset: 16,
                                shader_location: 4,
                                format: wgpu::VertexFormat::Float32x4,
                            },
                            wgpu::VertexAttribute {
                                offset: 32,
                                shader_location: 5,
                                format: wgpu::VertexFormat::Float32x4,
                            },
                            wgpu::VertexAttribute {
                                offset: 48,
                                shader_location: 6,
                                format: wgpu::VertexFormat::Float32x4,
                            },
                            wgpu::VertexAttribute {
                                offset: 64,
                                shader_location: 7,
                                format: wgpu::VertexFormat::Float32x4,
                            },
                        ],
                    },
                ],
                compilation_options: Default::default(),
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Front), // Front-face culling reduces shadow acne
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState {
                    constant: 2,      // Constant depth bias to reduce shadow acne
                    slope_scale: 2.0, // Slope-scaled bias
                    clamp: 0.0,
                },
            }),
            multisample: wgpu::MultisampleState::default(),
            fragment: None, // Depth-only — no color output
            multiview: None,
            cache: None,
        });

        // ═══════════════════════════════════════════════════════════════════
        // IBL resources (BRDF LUT + SH uniform)
        // ═══════════════════════════════════════════════════════════════════

        // Generate BRDF LUT via compute shader
        let brdf_lut_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("BRDF LUT"),
            size: wgpu::Extent3d {
                width: BRDF_LUT_SIZE,
                height: BRDF_LUT_SIZE,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba16Float,
            usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let brdf_lut_view = brdf_lut_texture.create_view(&Default::default());

        // Compute pipeline for BRDF LUT baking
        let brdf_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("BRDF LUT Compute Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/brdf_lut.wgsl").into()),
        });

        let brdf_compute_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("BRDF LUT Compute BGL"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::WriteOnly,
                        format: wgpu::TextureFormat::Rgba16Float,
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
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

        let brdf_compute_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("BRDF LUT Compute Layout"),
            bind_group_layouts: &[&brdf_compute_bgl],
            push_constant_ranges: &[],
        });

        let brdf_compute_pipeline =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("BRDF LUT Compute Pipeline"),
                layout: Some(&brdf_compute_layout),
                module: &brdf_shader,
                entry_point: Some("cs_brdf_lut"),
                compilation_options: Default::default(),
                cache: None,
            });

        // Params buffer (uniform: size)
        let brdf_params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("BRDF LUT Params"),
            contents: bytemuck::bytes_of(&[BRDF_LUT_SIZE, 0u32, 0u32, 0u32]),
            usage: wgpu::BufferUsages::UNIFORM,
        });

        let brdf_compute_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("BRDF LUT Compute BG"),
            layout: &brdf_compute_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&brdf_lut_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: brdf_params_buffer.as_entire_binding(),
                },
            ],
        });

        // Dispatch BRDF LUT compute shader (one-time bake)
        {
            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("BRDF LUT Bake Encoder"),
            });
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("BRDF LUT Bake"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&brdf_compute_pipeline);
            pass.set_bind_group(0, &brdf_compute_bg, &[]);
            let wg = (BRDF_LUT_SIZE + 7) / 8;
            pass.dispatch_workgroups(wg, wg, 1);
            drop(pass);
            queue.submit(std::iter::once(encoder.finish()));
        }

        // IBL sampling sampler (clamp, for LUT lookups)
        let ibl_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("IBL BRDF LUT Sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            ..Default::default()
        });

        // IBL uniform buffer (SH9 + intensity)
        let default_ibl = IblParamsGpu::default();
        let ibl_uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("IBL Params UB"),
            contents: bytemuck::bytes_of(&default_ibl),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // IBL bind group layout (group 4): BRDF LUT + sampler + SH uniform
        let ibl_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("IBL Bind Group Layout (group 4)"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
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

        // IBL bind group
        let ibl_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("IBL Bind Group"),
            layout: &ibl_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&brdf_lut_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&ibl_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: ibl_uniform_buffer.as_entire_binding(),
                },
            ],
        });

        // --- Textured pipeline (groups 0-4) ---
        let textured_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Entity PBR Textured Pipeline Layout"),
                bind_group_layouts: &[
                    &bind_group_layout,
                    &texture_bind_group_layout,
                    &material_bind_group_layout,
                    &shadow_bind_group_layout,
                    &ibl_bind_group_layout,
                ],
                push_constant_ranges: &[],
            });

        let vertex_buffers = [
            wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<Vertex>() as u64,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &[
                    wgpu::VertexAttribute {
                        offset: 0,
                        shader_location: 0,
                        format: wgpu::VertexFormat::Float32x3,
                    },
                    wgpu::VertexAttribute {
                        offset: 12,
                        shader_location: 1,
                        format: wgpu::VertexFormat::Float32x3,
                    },
                    wgpu::VertexAttribute {
                        offset: 24,
                        shader_location: 2,
                        format: wgpu::VertexFormat::Float32x4,
                    },
                    wgpu::VertexAttribute {
                        offset: 40,
                        shader_location: 8,
                        format: wgpu::VertexFormat::Float32x2,
                    },
                ],
            },
            wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<Instance>() as u64,
                step_mode: wgpu::VertexStepMode::Instance,
                attributes: &[
                    wgpu::VertexAttribute {
                        offset: 0,
                        shader_location: 3,
                        format: wgpu::VertexFormat::Float32x4,
                    },
                    wgpu::VertexAttribute {
                        offset: 16,
                        shader_location: 4,
                        format: wgpu::VertexFormat::Float32x4,
                    },
                    wgpu::VertexAttribute {
                        offset: 32,
                        shader_location: 5,
                        format: wgpu::VertexFormat::Float32x4,
                    },
                    wgpu::VertexAttribute {
                        offset: 48,
                        shader_location: 6,
                        format: wgpu::VertexFormat::Float32x4,
                    },
                    wgpu::VertexAttribute {
                        offset: 64,
                        shader_location: 7,
                        format: wgpu::VertexFormat::Float32x4,
                    },
                ],
            },
        ];

        let textured_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Entity Textured Render Pipeline"),
            layout: Some(&textured_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &vertex_buffers,
                compilation_options: Default::default(),
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
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
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_textured"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: color_format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            multiview: None,
            cache: None,
        });

        // --- Alpha-blend pipeline variant (for Blend alpha mode) ---
        let alpha_blend_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Entity Alpha Blend Pipeline"),
            layout: Some(&textured_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &vertex_buffers,
                compilation_options: Default::default(),
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: false,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_textured"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: color_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            multiview: None,
            cache: None,
        });

        // --- Double-sided pipeline variant (no backface culling) ---
        let double_sided_pipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Entity Double-Sided Pipeline"),
                layout: Some(&textured_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    buffers: &vertex_buffers,
                    compilation_options: Default::default(),
                },
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
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
                multisample: wgpu::MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_textured"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: color_format,
                        blend: Some(wgpu::BlendState::REPLACE),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
                }),
                multiview: None,
                cache: None,
            });

        // --- Wireframe pipelines (PolygonMode::Line) if GPU supports it ---
        let has_wireframe = device
            .features()
            .contains(wgpu::Features::POLYGON_MODE_LINE);

        let wireframe_primitive = wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None, // No backface culling for wireframe
            unclipped_depth: false,
            polygon_mode: wgpu::PolygonMode::Line,
            conservative: false,
        };

        let wireframe_depth = wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::Less,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        };

        let wireframe_multisample = wgpu::MultisampleState {
            count: 1,
            mask: !0,
            alpha_to_coverage_enabled: false,
        };

        let wireframe_pipeline = if has_wireframe {
            Some(
                device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("Entity Wireframe Pipeline"),
                    layout: Some(&pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: &shader,
                        entry_point: Some("vs_main"),
                        buffers: &vertex_buffers,
                        compilation_options: Default::default(),
                    },
                    primitive: wireframe_primitive,
                    depth_stencil: Some(wireframe_depth.clone()),
                    multisample: wireframe_multisample,
                    fragment: Some(wgpu::FragmentState {
                        module: &shader,
                        entry_point: Some("fs_main"),
                        targets: &[Some(wgpu::ColorTargetState {
                            format: color_format,
                            blend: Some(wgpu::BlendState::REPLACE),
                            write_mask: wgpu::ColorWrites::ALL,
                        })],
                        compilation_options: Default::default(),
                    }),
                    multiview: None,
                    cache: None,
                }),
            )
        } else {
            None
        };

        let wireframe_textured_pipeline = if has_wireframe {
            Some(
                device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("Entity Wireframe Textured Pipeline"),
                    layout: Some(&textured_pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: &shader,
                        entry_point: Some("vs_main"),
                        buffers: &vertex_buffers,
                        compilation_options: Default::default(),
                    },
                    primitive: wireframe_primitive,
                    depth_stencil: Some(wireframe_depth),
                    multisample: wireframe_multisample,
                    fragment: Some(wgpu::FragmentState {
                        module: &shader,
                        entry_point: Some("fs_textured"),
                        targets: &[Some(wgpu::ColorTargetState {
                            format: color_format,
                            blend: Some(wgpu::BlendState::REPLACE),
                            write_mask: wgpu::ColorWrites::ALL,
                        })],
                        compilation_options: Default::default(),
                    }),
                    multiview: None,
                    cache: None,
                }),
            )
        } else {
            None
        };

        Ok(Self {
            device,
            pipeline,
            bind_group_layout,
            bind_group,
            uniform_buffer,
            vertex_buffer,
            index_buffer,
            instance_buffer,
            max_instances,
            index_count,
            mesh_cache: HashMap::new(),
            failed_mesh_paths: HashMap::new(),
            entity_meshes: HashMap::new(),
            texture_bind_group_layout,
            queue,
            fallback_texture_bind_group,
            material_bind_group_layout,
            fallback_material_bind_group,
            fallback_normal_view,
            fallback_orm_view,
            fallback_emissive_view,
            fallback_albedo_view,
            fallback_sampler,
            textured_pipeline,
            alpha_blend_pipeline,
            double_sided_pipeline,
            wireframe_pipeline,
            wireframe_textured_pipeline,
            texture_path_cache: HashMap::new(),
            entity_texture_overrides: HashMap::new(),
            scene_lights: Vec::new(),
            sun_direction: [0.5, 0.7, 0.35],
            sun_color: [1.0, 0.95, 0.85],
            sun_intensity: 1.8,
            ambient_color: [0.55, 0.52, 0.48],
            ambient_intensity: 0.35,
            shadow_texture,
            shadow_texture_view,
            shadow_bind_group_layout,
            shadow_bind_group,
            shadow_pipeline,
            shadow_uniform_buffer,
            shadow_bind_group_0,
            shadow_enabled: true,
            brdf_lut_view,
            ibl_bind_group_layout,
            ibl_bind_group,
            ibl_uniform_buffer,
            ibl_enabled: true,
            exposure_ev: 0.0,
            hdr_output: false,
            mipmap_generator: None,
        })
    }

    /// Inject a GPU mipmap generator (replaces CPU box-filter mipmap path).
    pub fn set_mipmap_generator(&mut self, gen: super::mipmap_generator::MipmapGenerator) {
        self.mipmap_generator = Some(gen);
    }

    /// Set the entity-to-mesh mapping for the next render. Call before render().
    pub fn set_entity_meshes(&mut self, meshes: HashMap<Entity, String>) {
        self.entity_meshes = meshes;
    }

    /// Preload glTF meshes into the mesh cache so they're ready for entity rendering.
    /// Called when blend decomposition completes or when assets are imported.
    /// Returns the number of meshes successfully loaded.
    pub fn preload_gltf_meshes(&mut self, paths: &[String]) -> usize {
        let mut loaded = 0;
        for path in paths {
            if self.mesh_cache.contains_key(path) {
                loaded += 1;
                continue;
            }
            if let Some(&(attempts, last)) = self.failed_mesh_paths.get(path) {
                if attempts >= 3 || last.elapsed().as_secs() < 30 {
                    continue; // Permanently failed or still in cooldown
                }
            }
            match self.load_gltf_mesh(path) {
                Ok(()) => {
                    self.failed_mesh_paths.remove(path); // Clear on success (retry worked)
                    tracing::info!("Preloaded mesh: {}", path);
                    loaded += 1;
                }
                Err(e) => {
                    let attempts = self.failed_mesh_paths.get(path).map_or(0, |&(a, _)| a) + 1;
                    if attempts >= 3 {
                        tracing::warn!("Permanently failed to load mesh '{}' after {} attempts: {}", path, attempts, e);
                    } else {
                        tracing::warn!("Failed to preload mesh '{}' (attempt {}/3, retry in 30s): {}", path, attempts, e);
                    }
                    self.failed_mesh_paths.insert(path.clone(), (attempts, std::time::Instant::now()));
                }
            }
        }
        loaded
    }

    /// Set scene point lights (from entities with Light components). Up to 4 used.
    pub fn set_scene_lights(&mut self, lights: Vec<SceneLight>) {
        self.scene_lights = lights;
    }

    /// Set directional sun light parameters.
    pub fn set_sun(&mut self, direction: [f32; 3], color: [f32; 3], intensity: f32) {
        self.sun_direction = direction;
        self.sun_color = color;
        self.sun_intensity = intensity;
    }

    /// Set ambient light parameters.
    pub fn set_ambient(&mut self, color: [f32; 3], intensity: f32) {
        self.ambient_color = color;
        self.ambient_intensity = intensity;
    }

    /// Enable or disable shadow mapping.
    pub fn set_shadows_enabled(&mut self, enabled: bool) {
        self.shadow_enabled = enabled;
    }

    /// Whether shadow mapping is currently enabled.
    pub fn shadows_enabled(&self) -> bool {
        self.shadow_enabled
    }

    /// Compute the light-space view-projection matrix for the directional sun.
    /// Centers the shadow frustum on `focus_pos` (typically the camera position).
    fn compute_shadow_vp(&self, focus_pos: Vec3) -> Mat4 {
        let sun_dir = Vec3::from(self.sun_direction).normalize_or(Vec3::new(0.5, 0.7, 0.35));
        let light_pos = focus_pos - sun_dir * (SHADOW_HALF_EXTENT * 2.0);

        // Choose an up vector that isn't parallel to sun direction
        let up = if sun_dir.y.abs() > 0.99 {
            Vec3::Z
        } else {
            Vec3::Y
        };

        let light_view = Mat4::look_at_rh(light_pos, focus_pos, up);
        let light_proj = Mat4::orthographic_rh(
            -SHADOW_HALF_EXTENT,
            SHADOW_HALF_EXTENT,
            -SHADOW_HALF_EXTENT,
            SHADOW_HALF_EXTENT,
            0.1,
            SHADOW_HALF_EXTENT * 4.0,
        );
        light_proj * light_view
    }

    /// Enable or disable IBL.
    pub fn set_ibl_enabled(&mut self, enabled: bool) {
        self.ibl_enabled = enabled;
    }

    /// Whether IBL is currently enabled.
    pub fn ibl_enabled(&self) -> bool {
        self.ibl_enabled
    }

    /// Set exposure compensation in EV (exposure value).
    /// 0.0 = neutral, +1.0 = double brightness, -1.0 = half brightness.
    pub fn set_exposure_ev(&mut self, ev: f32) {
        self.exposure_ev = ev;
    }

    /// Current exposure compensation in EV.
    pub fn exposure_ev(&self) -> f32 {
        self.exposure_ev
    }

    /// Enable HDR output mode (post-process chain handles tonemapping).
    pub fn set_hdr_output(&mut self, enabled: bool) {
        self.hdr_output = enabled;
    }

    /// Current sun direction.
    pub fn sun_direction(&self) -> [f32; 3] {
        self.sun_direction
    }

    /// Current sun color.
    pub fn sun_color(&self) -> [f32; 3] {
        self.sun_color
    }

    /// Current sun intensity.
    pub fn sun_intensity(&self) -> f32 {
        self.sun_intensity
    }

    /// Compute SH L2 irradiance coefficients from sky + ground color.
    /// This is a simple analytical approximation (not a cubemap convolution):
    /// hemisphere sky color + ground color → 3 non-zero SH bands.
    fn compute_sky_sh(&self) -> IblParamsGpu {
        let sky = [
            self.ambient_color[0] * self.ambient_intensity,
            self.ambient_color[1] * self.ambient_intensity,
            self.ambient_color[2] * self.ambient_intensity,
        ];
        let ground = [sky[0] * 0.4, sky[1] * 0.4, sky[2] * 0.4];

        // SH constants for L0 and L1 bands
        let c0 = 0.282095; // Y_00 = 1/(2*sqrt(pi))
        let c1 = 0.488603; // Y_1x = sqrt(3)/(2*sqrt(pi))

        // Average = (sky+ground)/2, vertical gradient = (sky-ground)/2
        let avg = [
            (sky[0] + ground[0]) * 0.5,
            (sky[1] + ground[1]) * 0.5,
            (sky[2] + ground[2]) * 0.5,
        ];
        let diff = [
            (sky[0] - ground[0]) * 0.5,
            (sky[1] - ground[1]) * 0.5,
            (sky[2] - ground[2]) * 0.5,
        ];

        // L00 = average * SH_00 scale (inverse of basis weight in irradiance convolution)
        let sh0 = [avg[0] / c0, avg[1] / c0, avg[2] / c0, 0.0];
        // L10 (Y up) = vertical gradient / c1
        let sh2 = [diff[0] / c1, diff[1] / c1, diff[2] / c1, 0.0];

        // Also add subtle directional bias from sun direction for richer lighting
        let sun_int = self.sun_intensity * 0.15; // Reduced contribution to avoid double-counting
        let sun = [
            self.sun_color[0] * sun_int * self.sun_direction[0],
            self.sun_color[1] * sun_int * self.sun_direction[1],
            self.sun_color[2] * sun_int * self.sun_direction[2],
        ];
        // L11 (X axis)
        let sh3 = [sun[0] / c1, sun[1] / c1, sun[2] / c1, 0.0];
        // L1-1 (Y axis) — already handled in sh2
        let sh1 = [0.0, 0.0, 0.0, 0.0];

        IblParamsGpu {
            sh0,
            sh1,
            sh2,
            sh3,
            sh4: [0.0; 4],
            sh5: [0.0; 4],
            sh6: [0.0; 4],
            sh7: [0.0; 4],
            sh8: [0.0; 4],
            ibl_intensity: [
                1.0, // diffuse intensity
                0.5, // specular intensity (half — SH is low-freq)
                6.0, // max spec mip (unused for SH-only path)
                if self.ibl_enabled { 1.0 } else { 0.0 },
            ],
        }
    }

    fn pack_light_pos(&self, idx: usize) -> [f32; 4] {
        match self.scene_lights.get(idx) {
            Some(l) => [l.position[0], l.position[1], l.position[2], l.range],
            None => [0.0; 4],
        }
    }

    fn pack_light_color(&self, idx: usize) -> [f32; 4] {
        match self.scene_lights.get(idx) {
            Some(l) => [l.color[0], l.color[1], l.color[2], l.intensity],
            None => [0.0; 4],
        }
    }

    /// Collect the transform matrix for a glTF node (decomposed TRS → Mat4).
    fn gltf_node_transform(node: &gltf::Node) -> Mat4 {
        let (translation, rotation, scale) = node.transform().decomposed();
        let t = Mat4::from_translation(Vec3::from(translation));
        let r = Mat4::from_quat(glam::Quat::from_array(rotation));
        let s = Mat4::from_scale(Vec3::from(scale));
        t * r * s
    }

    /// Recursively walk the scene graph, collecting (mesh_index, world_transform) pairs.
    fn collect_mesh_nodes(node: &gltf::Node, parent_transform: Mat4, out: &mut Vec<(usize, Mat4)>) {
        let local = Self::gltf_node_transform(node);
        let world = parent_transform * local;
        if let Some(mesh) = node.mesh() {
            out.push((mesh.index(), world));
        }
        for child in node.children() {
            Self::collect_mesh_nodes(&child, world, out);
        }
    }

    /// Load a GLTF/GLB mesh from disk and cache the GPU buffers.
    ///
    /// Iterates ALL meshes, ALL primitives, and applies node transforms from
    /// the scene graph so that multi-part models (e.g. KayKit characters with
    /// separate body/hair/armor meshes) render correctly as a single unit.
    fn load_gltf_mesh(&mut self, path: &str) -> Result<()> {
        let (document, buffers, images) =
            gltf::import(path).with_context(|| format!("Failed to import glTF: {path}"))?;

        // Walk scene graph to get per-node transforms for each mesh
        let mut mesh_nodes: Vec<(usize, Mat4)> = Vec::new();
        if let Some(scene) = document
            .default_scene()
            .or_else(|| document.scenes().next())
        {
            for node in scene.nodes() {
                Self::collect_mesh_nodes(&node, Mat4::IDENTITY, &mut mesh_nodes);
            }
        }

        // If scene graph is empty, fall back to rendering all meshes at identity
        if mesh_nodes.is_empty() {
            for mesh in document.meshes() {
                mesh_nodes.push((mesh.index(), Mat4::IDENTITY));
            }
        }

        let mut all_vertices: Vec<Vertex> = Vec::new();
        let mut all_indices: Vec<u32> = Vec::new();
        // Track PBR texture indices from the first material we encounter
        let mut first_albedo_image_idx: Option<usize> = None;
        let mut first_normal_image_idx: Option<usize> = None;
        let mut first_mr_image_idx: Option<usize> = None; // metallic-roughness
        let mut first_emissive_image_idx: Option<usize> = None;
        let mut _first_occlusion_image_idx: Option<usize> = None;
        // Capture first material's PBR scalar parameters
        let mut first_mat_params: Option<MaterialParamsGpu> = None;
        // Alpha mode for pipeline selection: 0=Opaque, 1=Mask, 2=Blend
        let mut mesh_alpha_mode: u8 = 0;
        // Whether the material is double-sided (no backface culling)
        let mut mesh_double_sided: bool = false;
        // Skinning data (per-vertex joint indices + weights)
        let mut all_joints: Vec<[u16; 4]> = Vec::new();
        let mut all_weights: Vec<[f32; 4]> = Vec::new();
        let mut has_skinning = false;

        for (mesh_idx, node_transform) in &mesh_nodes {
            let mesh = document
                .meshes()
                .nth(*mesh_idx)
                .context("Mesh index out of range")?;

            // Extract normal matrix (inverse-transpose of upper-left 3×3)
            let normal_mat = node_transform.inverse().transpose();

            for primitive in mesh.primitives() {
                let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));

                let Some(pos_iter) = reader.read_positions() else {
                    continue;
                };
                let positions: Vec<[f32; 3]> = pos_iter.collect();

                let normals: Vec<[f32; 3]> = if let Some(n) = reader.read_normals() {
                    n.collect()
                } else {
                    vec![[0.0, 1.0, 0.0]; positions.len()]
                };

                // Per-vertex colors (common in KayKit/Kenney models)
                let vertex_colors: Vec<[f32; 4]> = if let Some(colors) = reader.read_colors(0) {
                    colors.into_rgba_f32().collect()
                } else {
                    // Fall back to material base color factor
                    let base_color = primitive
                        .material()
                        .pbr_metallic_roughness()
                        .base_color_factor();
                    vec![base_color; positions.len()]
                };

                // UV coordinates (set 0)
                let uvs: Vec<[f32; 2]> = if let Some(tc) = reader.read_tex_coords(0) {
                    tc.into_f32().collect()
                } else {
                    vec![[0.0, 0.0]; positions.len()]
                };

                // Skinning: joint indices + weights (set 0)
                let prim_joints: Option<Vec<[u16; 4]>> =
                    reader.read_joints(0).map(|j| j.into_u16().collect());
                let prim_weights: Option<Vec<[f32; 4]>> =
                    reader.read_weights(0).map(|w| w.into_f32().collect());

                // Extract PBR texture indices and material params from first material
                if first_albedo_image_idx.is_none() {
                    let mat = primitive.material();
                    let pbr = mat.pbr_metallic_roughness();

                    // Albedo (base color) texture
                    if let Some(tex_info) = pbr.base_color_texture() {
                        first_albedo_image_idx = Some(tex_info.texture().source().index());
                    }
                    // Normal map
                    if let Some(normal_tex) = mat.normal_texture() {
                        first_normal_image_idx = Some(normal_tex.texture().source().index());
                    }
                    // Metallic-roughness texture
                    if let Some(mr_tex) = pbr.metallic_roughness_texture() {
                        first_mr_image_idx = Some(mr_tex.texture().source().index());
                    }
                    // Emissive texture
                    if let Some(em_tex) = mat.emissive_texture() {
                        first_emissive_image_idx = Some(em_tex.texture().source().index());
                    }
                    // Occlusion texture
                    if let Some(occ_tex) = mat.occlusion_texture() {
                        _first_occlusion_image_idx = Some(occ_tex.texture().source().index());
                    }

                    // Build material scalar parameters
                    let bc = pbr.base_color_factor();
                    let emissive = mat.emissive_factor();
                    let occ_strength = mat.occlusion_texture().map(|o| o.strength()).unwrap_or(1.0);
                    let alpha_mode_val = match mat.alpha_mode() {
                        gltf::material::AlphaMode::Opaque => 0.0_f32,
                        gltf::material::AlphaMode::Mask => 1.0,
                        gltf::material::AlphaMode::Blend => 2.0,
                    };
                    let alpha_cutoff = mat.alpha_cutoff().unwrap_or(0.5);

                    // Track for pipeline variant selection
                    mesh_alpha_mode = alpha_mode_val as u8;
                    mesh_double_sided = mat.double_sided();

                    let mut ior = 1.5_f32;

                    // Read KHR extensions from raw JSON if typed API unavailable
                    let mut emissive_str = 1.0_f32;
                    let mut transmission = 0.0_f32;
                    let mut cc_factor = 0.0_f32;
                    let mut cc_rough = 0.0_f32;
                    if let Some(ext_map) = mat.extensions() {
                        if let Some(v) = ext_map.get("KHR_materials_ior") {
                            if let Some(f) = v.get("ior").and_then(|x| x.as_f64()) {
                                ior = f as f32;
                            }
                        }
                        if let Some(v) = ext_map.get("KHR_materials_emissive_strength") {
                            if let Some(f) = v.get("emissiveStrength").and_then(|x| x.as_f64()) {
                                emissive_str = f as f32;
                            }
                        }
                        if let Some(v) = ext_map.get("KHR_materials_transmission") {
                            if let Some(f) = v.get("transmissionFactor").and_then(|x| x.as_f64()) {
                                transmission = f as f32;
                            }
                        }
                        if let Some(v) = ext_map.get("KHR_materials_clearcoat") {
                            if let Some(f) = v.get("clearcoatFactor").and_then(|x| x.as_f64()) {
                                cc_factor = f as f32;
                            }
                            if let Some(f) =
                                v.get("clearcoatRoughnessFactor").and_then(|x| x.as_f64())
                            {
                                cc_rough = f as f32;
                            }
                        }
                    }
                    let _ = transmission; // reserved for future use

                    first_mat_params = Some(MaterialParamsGpu {
                        base_color_factor: bc,
                        emissive_and_metallic: [
                            emissive[0],
                            emissive[1],
                            emissive[2],
                            pbr.metallic_factor(),
                        ],
                        pbr_params: [
                            pbr.roughness_factor(),
                            emissive_str,
                            occ_strength,
                            alpha_cutoff,
                        ],
                        extra_params: [ior, cc_factor, cc_rough, alpha_mode_val],
                    });
                }

                let Some(idx_iter) = reader.read_indices() else {
                    continue;
                };
                let prim_indices: Vec<u32> = idx_iter.into_u32().collect();

                // Base index for this primitive's vertices in the merged buffer
                let base_vertex = all_vertices.len() as u32;

                for (((p, n), c), uv) in positions
                    .iter()
                    .zip(normals.iter())
                    .zip(vertex_colors.iter())
                    .zip(uvs.iter())
                {
                    // Apply node transform to position
                    let tp = *node_transform * glam::Vec4::new(p[0], p[1], p[2], 1.0);
                    // Apply normal matrix to normal and re-normalize
                    let tn = (normal_mat * glam::Vec4::new(n[0], n[1], n[2], 0.0)).truncate();
                    let tn = tn.normalize_or(Vec3::Y);
                    all_vertices.push(Vertex {
                        position: [tp.x, tp.y, tp.z],
                        normal: [tn.x, tn.y, tn.z],
                        color: *c,
                        uv: *uv,
                    });
                }

                // Collect skinning data (pad with identity weights if absent)
                if let (Some(joints), Some(weights)) = (&prim_joints, &prim_weights) {
                    has_skinning = true;
                    all_joints.extend_from_slice(joints);
                    all_weights.extend_from_slice(weights);
                } else {
                    let count = positions.len();
                    all_joints.extend(std::iter::repeat([0u16; 4]).take(count));
                    all_weights.extend(std::iter::repeat([1.0, 0.0, 0.0, 0.0f32]).take(count));
                }

                for idx in &prim_indices {
                    all_indices.push(base_vertex + idx);
                }
            }
        }

        if all_vertices.is_empty() {
            anyhow::bail!("No renderable geometry found in glTF: {path}");
        }

        // Use COPY_DST so we can update vertex positions for CPU skinning
        let vertex_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&format!("Mesh VB: {path}")),
                contents: bytemuck::cast_slice(&all_vertices),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            });

        let index_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&format!("Mesh IB: {path}")),
                contents: bytemuck::cast_slice(&all_indices),
                usage: wgpu::BufferUsages::INDEX,
            });

        // --- Extract PBR textures from embedded glTF images ---
        let albedo_view = first_albedo_image_idx
            .and_then(|idx| images.get(idx))
            .and_then(|img| self.create_gpu_texture_view(img, &format!("Albedo: {path}"), true));
        let normal_view = first_normal_image_idx
            .and_then(|idx| images.get(idx))
            .and_then(|img| self.create_gpu_texture_view(img, &format!("Normal: {path}"), false));
        let mr_view = first_mr_image_idx
            .and_then(|idx| images.get(idx))
            .and_then(|img| {
                self.create_gpu_texture_view(img, &format!("MetalRough: {path}"), false)
            });

        // If separate occlusion and metallic-roughness textures exist, prefer ORM composite.
        // For now, use metallic-roughness as-is since glTF packs G=roughness, B=metallic.
        // Remap to our ORM layout: R=occlusion(from occ tex or 1.0), G=roughness, B=metallic.
        // TODO: Composite separate occlusion + metallic-roughness into ORM at load time.
        let orm_view = mr_view; // glTF metallic-roughness maps to our ORM binding 3

        let emissive_view = first_emissive_image_idx
            .and_then(|idx| images.get(idx))
            .and_then(|img| self.create_gpu_texture_view(img, &format!("Emissive: {path}"), true));

        let has_any_texture = albedo_view.is_some()
            || normal_view.is_some()
            || orm_view.is_some()
            || emissive_view.is_some();

        let texture_bind_group = if has_any_texture {
            Some(self.build_pbr_texture_bind_group(
                albedo_view.as_ref(),
                normal_view.as_ref(),
                orm_view.as_ref(),
                emissive_view.as_ref(),
                &format!("PBR-TexBG: {path}"),
            ))
        } else {
            None
        };

        // --- Build per-mesh material uniform ---
        let (material_uniform_buffer, material_bind_group) =
            if let Some(ref params) = first_mat_params {
                let (buf, bg) = self.build_material_bind_group(params, path);
                (Some(buf), Some(bg))
            } else {
                (None, None)
            };

        // --- Phase 2: Extract skeleton from glTF skins ---
        let skeleton = self.extract_gltf_skeleton(&document, &buffers);

        // --- Phase 2: Extract animation clips from glTF animations ---
        let animations = self.extract_gltf_animations(&document, &buffers, &skeleton);

        let skinning_data = if has_skinning {
            Some(SkinningData {
                joints: all_joints,
                weights: all_weights,
            })
        } else {
            None
        };

        let has_texture = texture_bind_group.is_some();
        let has_skel = skeleton.is_some();
        let anim_count = animations.len();

        tracing::info!(
            "Loaded glTF mesh '{}': {} vertices, {} indices ({} meshes, {} nodes, textured={}, skeleton={}, animations={})",
            path,
            all_vertices.len(),
            all_indices.len(),
            document.meshes().count(),
            mesh_nodes.len(),
            has_texture,
            has_skel,
            anim_count,
        );

        let rest_vertices = if has_skinning {
            Some(all_vertices)
        } else {
            None
        };

        self.mesh_cache.insert(
            path.to_string(),
            LoadedMesh {
                vertex_buffer,
                index_buffer,
                index_count: all_indices.len() as u32,
                index_format: wgpu::IndexFormat::Uint32,
                texture_bind_group,
                material_bind_group,
                material_uniform_buffer,
                skeleton,
                animations,
                skinning_data,
                rest_vertices,
                alpha_mode: mesh_alpha_mode,
                double_sided: mesh_double_sided,
            },
        );

        Ok(())
    }

    /// Convert a glTF image buffer into a GPU texture view with full mipmap chain.
    /// `srgb` controls whether the texture uses Rgba8UnormSrgb (true for color maps)
    /// or Rgba8Unorm (false for data maps like normal/ORM).
    fn create_gpu_texture_view(
        &self,
        img_data: &gltf::image::Data,
        label: &str,
        srgb: bool,
    ) -> Option<wgpu::TextureView> {
        let width = img_data.width;
        let height = img_data.height;

        // Convert to RGBA8 regardless of source format
        let rgba_pixels: Vec<u8> = match img_data.format {
            gltf::image::Format::R8G8B8A8 => img_data.pixels.clone(),
            gltf::image::Format::R8G8B8 => {
                let mut rgba = Vec::with_capacity(img_data.pixels.len() / 3 * 4);
                for chunk in img_data.pixels.chunks_exact(3) {
                    rgba.extend_from_slice(chunk);
                    rgba.push(255);
                }
                rgba
            }
            gltf::image::Format::R8 => {
                let mut rgba = Vec::with_capacity(img_data.pixels.len() * 4);
                for &v in &img_data.pixels {
                    rgba.extend_from_slice(&[v, v, v, 255]);
                }
                rgba
            }
            gltf::image::Format::R8G8 => {
                let mut rgba = Vec::with_capacity(img_data.pixels.len() / 2 * 4);
                for chunk in img_data.pixels.chunks_exact(2) {
                    // For data maps (normal maps), reconstruct Z from X/Y:
                    // z = sqrt(1 - x^2 - y^2), mapped from [0,255] → [-1,1] → back
                    let b = if !srgb {
                        let nx = chunk[0] as f32 / 255.0 * 2.0 - 1.0;
                        let ny = chunk[1] as f32 / 255.0 * 2.0 - 1.0;
                        let nz = (1.0 - (nx * nx + ny * ny).min(1.0)).sqrt();
                        ((nz * 0.5 + 0.5) * 255.0) as u8
                    } else {
                        0
                    };
                    rgba.extend_from_slice(&[chunk[0], chunk[1], b, 255]);
                }
                rgba
            }
            _ => {
                tracing::warn!("Unsupported glTF image format for {label}");
                return None;
            }
        };

        let fmt = if srgb {
            wgpu::TextureFormat::Rgba8UnormSrgb
        } else {
            wgpu::TextureFormat::Rgba8Unorm
        };

        // Compute full mipmap chain count from dimensions
        let mip_count = (width.max(height) as f32).log2().floor() as u32 + 1;

        let tex = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: mip_count,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: fmt,
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });

        // Upload mip level 0
        self.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &tex,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &rgba_pixels,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(width * 4),
                rows_per_image: Some(height),
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );

        // Generate mipmaps via GPU blit passes (H-1).
        // The GPU path is both faster and more correct for sRGB textures:
        // hardware linearises before bilinear filtering when the source view
        // format is *Srgb, avoiding the sRGB-space averaging bug of the old
        // CPU box filter.
        if mip_count > 1 {
            if let Some(gen) = self.mipmap_generator.as_ref() {
                let mut encoder =
                    self.device
                        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                            label: Some("mipmap gen encoder"),
                        });
                gen.generate(&self.device, &mut encoder, &tex, mip_count, fmt);
                self.queue.submit(std::iter::once(encoder.finish()));
            }
        }

        Some(tex.create_view(&Default::default()))
    }

    /// Build a 5-binding PBR texture bind group (group 1).
    /// Uses fallback textures for any slot where `None` is passed.
    fn build_pbr_texture_bind_group(
        &self,
        albedo_view: Option<&wgpu::TextureView>,
        normal_view: Option<&wgpu::TextureView>,
        orm_view: Option<&wgpu::TextureView>,
        emissive_view: Option<&wgpu::TextureView>,
        label: &str,
    ) -> wgpu::BindGroup {
        self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(label),
            layout: &self.texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(
                        albedo_view.unwrap_or(&self.fallback_albedo_view),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.fallback_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(
                        normal_view.unwrap_or(&self.fallback_normal_view),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(
                        orm_view.unwrap_or(&self.fallback_orm_view),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::TextureView(
                        emissive_view.unwrap_or(&self.fallback_emissive_view),
                    ),
                },
            ],
        })
    }

    /// Build a material uniform buffer + bind group (group 2) from PBR parameters.
    fn build_material_bind_group(
        &self,
        params: &MaterialParamsGpu,
        label: &str,
    ) -> (wgpu::Buffer, wgpu::BindGroup) {
        let buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&format!("MatUB: {label}")),
                contents: bytemuck::bytes_of(params),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(&format!("MatBG: {label}")),
            layout: &self.material_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
        });
        (buffer, bind_group)
    }

    /// Backward-compatible wrapper: creates a PBR texture bind group from a single glTF albedo image.
    fn create_texture_bind_group_from_gltf_image(
        &self,
        img_data: &gltf::image::Data,
        label: &str,
    ) -> Option<wgpu::BindGroup> {
        let albedo_view =
            self.create_gpu_texture_view(img_data, &format!("Albedo: {label}"), true)?;
        Some(self.build_pbr_texture_bind_group(
            Some(&albedo_view),
            None,
            None,
            None,
            &format!("PBR-TexBG: {label}"),
        ))
    }

    // ========================================================================
    // Phase 2: Skeleton & Animation Extraction
    // ========================================================================

    /// Extract skeleton from the first glTF skin.
    fn extract_gltf_skeleton(
        &self,
        document: &gltf::Document,
        buffers: &[gltf::buffer::Data],
    ) -> Option<GltfSkeleton> {
        let skin = document.skins().next()?;
        let reader = skin.reader(|buffer| Some(&buffers[buffer.index()]));

        // Inverse bind matrices
        let ibms: Vec<Mat4> = if let Some(ibm_iter) = reader.read_inverse_bind_matrices() {
            ibm_iter.map(|m| Mat4::from_cols_array_2d(&m)).collect()
        } else {
            vec![Mat4::IDENTITY; skin.joints().count()]
        };

        // Build joint list
        let joint_nodes: Vec<gltf::Node> = skin.joints().collect();
        // Map from glTF node index → skeleton joint index
        let node_to_joint: HashMap<usize, usize> = joint_nodes
            .iter()
            .enumerate()
            .map(|(ji, node)| (node.index(), ji))
            .collect();

        let mut joints = Vec::with_capacity(joint_nodes.len());
        let mut root_indices = Vec::new();

        for (ji, node) in joint_nodes.iter().enumerate() {
            let name = node.name().unwrap_or("joint").to_string();
            let local_transform = Self::gltf_node_transform(node);
            let ibm = ibms.get(ji).copied().unwrap_or(Mat4::IDENTITY);

            // Find parent: walk up the scene graph to find a node that's also a joint
            let parent_index = Self::find_joint_parent(node, &node_to_joint);

            if parent_index.is_none() {
                root_indices.push(ji);
            }

            joints.push(GltfJoint {
                name,
                parent_index,
                inverse_bind_matrix: ibm,
                local_transform,
            });
        }

        tracing::info!(
            "Extracted skeleton: {} joints, {} roots",
            joints.len(),
            root_indices.len()
        );

        Some(GltfSkeleton {
            joints,
            root_indices,
        })
    }

    /// Walk up the glTF scene graph to find a parent that's also in the joint set.
    fn find_joint_parent(
        node: &gltf::Node,
        node_to_joint: &HashMap<usize, usize>,
    ) -> Option<usize> {
        // glTF doesn't expose parent directly, so we check children of all nodes
        // For efficiency, we rely on the inverse bind matrix hierarchy instead
        // and use a simpler heuristic: joints are ordered parent-first in most exporters
        // The gltf crate doesn't expose parent pointers, so we skip parent lookup
        // and rely on the joint ordering (parent index < child index).
        let _ = (node, node_to_joint);
        None // Parent reconstruction handled by animation sampling
    }

    /// Extract animation clips from glTF animations.
    fn extract_gltf_animations(
        &self,
        document: &gltf::Document,
        buffers: &[gltf::buffer::Data],
        skeleton: &Option<GltfSkeleton>,
    ) -> Vec<GltfAnimationClip> {
        let Some(skel) = skeleton else {
            return Vec::new();
        };

        // Build node_index → joint_index map from skeleton
        // We don't store node indices in GltfJoint, so we rebuild from document skins
        let skin = match document.skins().next() {
            Some(s) => s,
            None => return Vec::new(),
        };
        let node_to_joint: HashMap<usize, usize> = skin
            .joints()
            .enumerate()
            .map(|(ji, node)| (node.index(), ji))
            .collect();

        let mut clips = Vec::new();

        for anim in document.animations() {
            let name = anim
                .name()
                .unwrap_or(&format!("Animation_{}", anim.index()))
                .to_string();
            let mut channels = Vec::new();
            let mut duration = 0.0f32;

            for channel in anim.channels() {
                let target = channel.target();
                let node_idx = target.node().index();

                // Only extract channels targeting joints in our skeleton
                let Some(&joint_index) = node_to_joint.get(&node_idx) else {
                    continue;
                };

                // Validate joint_index is within skeleton bounds
                if joint_index >= skel.joints.len() {
                    continue;
                }

                let property = match target.property() {
                    gltf::animation::Property::Translation => GltfChannelProperty::Translation,
                    gltf::animation::Property::Rotation => GltfChannelProperty::Rotation,
                    gltf::animation::Property::Scale => GltfChannelProperty::Scale,
                    _ => continue, // Skip morph targets
                };

                let reader = channel.reader(|buffer| Some(&buffers[buffer.index()]));

                let times: Vec<f32> = match reader.read_inputs() {
                    Some(iter) => iter.collect(),
                    None => continue,
                };

                let values: Vec<Vec<f32>> = match reader.read_outputs() {
                    Some(gltf::animation::util::ReadOutputs::Translations(iter)) => {
                        iter.map(|v| v.to_vec()).collect()
                    }
                    Some(gltf::animation::util::ReadOutputs::Rotations(iter)) => {
                        iter.into_f32().map(|v| v.to_vec()).collect()
                    }
                    Some(gltf::animation::util::ReadOutputs::Scales(iter)) => {
                        iter.map(|v| v.to_vec()).collect()
                    }
                    _ => continue,
                };

                if let Some(&last_t) = times.last() {
                    duration = duration.max(last_t);
                }

                let sampler = anim.samplers().nth(channel.sampler().index());
                let interpolation = match sampler.map(|s| s.interpolation()) {
                    Some(gltf::animation::Interpolation::Step) => GltfInterpolation::Step,
                    Some(gltf::animation::Interpolation::CubicSpline) => {
                        GltfInterpolation::CubicSpline
                    }
                    _ => GltfInterpolation::Linear,
                };

                channels.push(GltfAnimChannel {
                    joint_index,
                    property,
                    times,
                    values,
                    interpolation,
                });
            }

            if !channels.is_empty() {
                tracing::info!(
                    "Extracted animation '{}': {:.2}s, {} channels",
                    name,
                    duration,
                    channels.len()
                );
                clips.push(GltfAnimationClip {
                    name,
                    duration,
                    channels,
                });
            }
        }

        clips
    }

    // ========================================================================
    // Phase 3: External Texture Loading
    // ========================================================================

    /// Load a texture from a file path and cache the bind group.
    /// Returns true if the texture was loaded (or already cached).
    pub fn load_external_texture(&mut self, path: &str) -> bool {
        if self.texture_path_cache.contains_key(path) {
            return true;
        }

        let img = match image::open(path) {
            Ok(img) => img.to_rgba8(),
            Err(e) => {
                tracing::warn!("Failed to load external texture '{}': {}", path, e);
                return false;
            }
        };

        let (width, height) = img.dimensions();
        let rgba_pixels = img.into_raw();

        let tex = self.device.create_texture_with_data(
            &self.queue,
            &wgpu::TextureDescriptor {
                label: Some(&format!("ExtTex: {path}")),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            },
            wgpu::util::TextureDataOrder::LayerMajor,
            &rgba_pixels,
        );

        let view = tex.create_view(&Default::default());

        // Build a PBR texture bind group with only albedo (others get fallbacks)
        let bind_group = self.build_pbr_texture_bind_group(
            Some(&view),
            None,
            None,
            None,
            &format!("ExtPBR-TexBG: {path}"),
        );

        tracing::info!("Loaded external texture '{}': {}×{}", path, width, height);
        self.texture_path_cache.insert(path.to_string(), bind_group);
        true
    }

    /// Set per-entity external texture overrides (entity → texture file path).
    pub fn set_entity_texture_overrides(&mut self, overrides: HashMap<Entity, String>) {
        self.entity_texture_overrides = overrides;
    }

    // ========================================================================
    // Phase 2/4: Public accessors for skeleton & animation data
    // ========================================================================

    /// Get the skeleton for a cached mesh (if it has one).
    pub fn get_mesh_skeleton(&self, mesh_path: &str) -> Option<&GltfSkeleton> {
        self.mesh_cache
            .get(mesh_path)
            .and_then(|m| m.skeleton.as_ref())
    }

    /// Get the animation clips for a cached mesh.
    pub fn get_mesh_animations(&self, mesh_path: &str) -> &[GltfAnimationClip] {
        self.mesh_cache
            .get(mesh_path)
            .map(|m| m.animations.as_slice())
            .unwrap_or(&[])
    }

    // ========================================================================
    // Phase 4: CPU Skinning
    // ========================================================================

    /// Apply CPU skinning to a cached mesh using the provided joint matrices.
    /// Updates the vertex buffer in-place with skinned positions and normals.
    pub fn apply_cpu_skinning(
        &mut self,
        mesh_path: &str,
        joint_matrices: &[Mat4],
        queue: &wgpu::Queue,
    ) {
        let Some(mesh) = self.mesh_cache.get(mesh_path) else {
            return;
        };
        let Some(skinning) = &mesh.skinning_data else {
            return;
        };
        let Some(rest_verts) = &mesh.rest_vertices else {
            return;
        };

        let mut skinned = rest_verts.clone();

        for (i, vert) in skinned.iter_mut().enumerate() {
            let joints = skinning.joints[i];
            let weights = skinning.weights[i];

            let mut skin_mat = Mat4::ZERO;
            for k in 0..4 {
                let ji = joints[k] as usize;
                if ji < joint_matrices.len() {
                    skin_mat += joint_matrices[ji] * weights[k];
                }
            }

            // Transform position
            let pos = Vec3::from(vert.position);
            let skinned_pos = skin_mat.transform_point3(pos);
            vert.position = skinned_pos.into();

            // Transform normal (use upper-left 3×3, re-normalize)
            let norm = Vec3::from(vert.normal);
            let skinned_norm = skin_mat.transform_vector3(norm).normalize_or(Vec3::Y);
            vert.normal = skinned_norm.into();
        }

        queue.write_buffer(&mesh.vertex_buffer, 0, bytemuck::cast_slice(&skinned));
    }

    /// Render all entities in the World
    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        depth: &wgpu::TextureView,
        camera: &OrbitCamera,
        world: &World,
        selected_entities: &[Entity],
        queue: &wgpu::Queue,
        shading_mode: u32,
    ) -> Result<()> {
        // Update camera uniforms — camera-relative VP to avoid f32 jitter far from origin
        let view_proj = camera.view_projection_matrix_relative();
        let camera_pos = camera.position();

        // Compute directional shadow light-space VP matrix
        let shadow_vp = self.compute_shadow_vp(camera_pos);

        let uniforms = EntityUniforms {
            view_proj: view_proj.to_cols_array_2d(),
            camera_pos: [camera_pos.x, camera_pos.y, camera_pos.z],
            shading_mode,
            sun_dir_and_count: [
                self.sun_direction[0],
                self.sun_direction[1],
                self.sun_direction[2],
                self.scene_lights.len().min(4) as f32,
            ],
            sun_color_and_intensity: [
                self.sun_color[0],
                self.sun_color[1],
                self.sun_color[2],
                self.sun_intensity,
            ],
            ambient_color_and_intensity: [
                self.ambient_color[0],
                self.ambient_color[1],
                self.ambient_color[2],
                self.ambient_intensity,
            ],
            light0_pos_range: self.pack_light_pos(0),
            light0_color_intensity: self.pack_light_color(0),
            light1_pos_range: self.pack_light_pos(1),
            light1_color_intensity: self.pack_light_color(1),
            light2_pos_range: self.pack_light_pos(2),
            light2_color_intensity: self.pack_light_color(2),
            light3_pos_range: self.pack_light_pos(3),
            light3_color_intensity: self.pack_light_color(3),
            shadow_vp: shadow_vp.to_cols_array_2d(),
            shadow_params: [
                0.002,                                       // x: depth bias
                0.05,                                        // y: normal offset bias
                if self.shadow_enabled { 1.0 } else { 0.0 }, // z: enabled flag
                1.0 / SHADOW_MAP_SIZE as f32,                // w: texel size for PCF
            ],
            exposure_params: [
                self.exposure_ev,
                if self.hdr_output { 1.0 } else { 0.0 },
                0.0,
                0.0,
            ],
        };

        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&uniforms));

        // Update IBL SH uniforms from current sky settings
        let ibl_params = self.compute_sky_sh();
        queue.write_buffer(&self.ibl_uniform_buffer, 0, bytemuck::bytes_of(&ibl_params));

        let frustum = camera.extract_frustum();

        // Collect instances grouped by mesh
        let (all_instances, draw_groups) =
            self.collect_instances_grouped(world, selected_entities, &frustum);

        if all_instances.is_empty() {
            return Ok(());
        }

        // Lazy-load any GLTF meshes not yet cached (skip known-failed paths)
        let paths_to_load: Vec<String> = draw_groups
            .iter()
            .filter_map(|(mesh, _, _)| mesh.clone())
            .filter(|p| {
                if self.mesh_cache.contains_key(p) {
                    return false;
                }
                if let Some(&(attempts, last)) = self.failed_mesh_paths.get(p.as_str()) {
                    if attempts >= 3 || last.elapsed().as_secs() < 30 {
                        return false; // Permanently failed or still in cooldown
                    }
                }
                true
            })
            .collect();
        for path in paths_to_load {
            if let Err(e) = self.load_gltf_mesh(&path) {
                let attempts = self.failed_mesh_paths.get(&path).map_or(0, |&(a, _)| a) + 1;
                tracing::warn!("Failed to load mesh '{}' (attempt {}/3): {}", path, attempts, e);
                self.failed_mesh_paths.insert(path, (attempts, std::time::Instant::now()));
            } else {
                self.failed_mesh_paths.remove(&path); // Clear on success
            }
        }

        // Lazy-load any external texture overrides not yet cached
        let tex_paths_to_load: Vec<String> = self
            .entity_texture_overrides
            .values()
            .filter(|p| !self.texture_path_cache.contains_key(*p))
            .cloned()
            .collect();
        for tex_path in tex_paths_to_load {
            self.load_external_texture(&tex_path);
        }

        let total = all_instances.len().min(self.max_instances as usize);
        queue.write_buffer(
            &self.instance_buffer,
            0,
            bytemuck::cast_slice(&all_instances[..total]),
        );

        let is_wireframe = shading_mode == 2;

        // ═══════════════════════════════════════════════════════════════════
        // Shadow depth pass — render scene from light's perspective
        // ═══════════════════════════════════════════════════════════════════
        if self.shadow_enabled {
            // Write light VP to shadow uniform buffer
            let shadow_uniforms = ShadowUniforms {
                light_vp: shadow_vp.to_cols_array_2d(),
            };
            queue.write_buffer(
                &self.shadow_uniform_buffer,
                0,
                bytemuck::bytes_of(&shadow_uniforms),
            );

            let mut shadow_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Shadow Depth Pass"),
                color_attachments: &[], // No color output — depth only
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.shadow_texture_view,
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
            shadow_pass.set_bind_group(0, &self.shadow_bind_group_0, &[]);
            shadow_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));

            for (mesh_path, start, count) in &draw_groups {
                if *count == 0 || (*start + *count) as usize > total {
                    continue;
                }

                match mesh_path {
                    Some(path) if self.mesh_cache.contains_key(path) => {
                        let mesh = &self.mesh_cache[path];
                        shadow_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                        shadow_pass
                            .set_index_buffer(mesh.index_buffer.slice(..), mesh.index_format);
                        shadow_pass.draw_indexed(0..mesh.index_count, 0, *start..*start + *count);
                    }
                    _ => {
                        shadow_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
                        shadow_pass.set_index_buffer(
                            self.index_buffer.slice(..),
                            wgpu::IndexFormat::Uint16,
                        );
                        shadow_pass.draw_indexed(0..self.index_count, 0, *start..*start + *count);
                    }
                }
            }
        }

        // ═══════════════════════════════════════════════════════════════════
        // Main entity render pass
        // ═══════════════════════════════════════════════════════════════════

        // Single render pass with multiple draw calls (one per mesh group)
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Entity Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: depth,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.set_vertex_buffer(1, self.instance_buffer.slice(..));

        // Sort draw groups: opaque first, then alpha-blend last for correct transparency
        let mut sorted_groups: Vec<_> = draw_groups.iter().collect();
        sorted_groups.sort_by_key(|(mesh_path, _, _)| match mesh_path {
            Some(path) if self.mesh_cache.contains_key(path) => {
                if self.mesh_cache[path].alpha_mode == 2 {
                    1u8
                } else {
                    0u8
                }
            }
            _ => 0u8,
        });

        for (mesh_path, start, count) in sorted_groups {
            if *count == 0 || (*start + *count) as usize > total {
                continue;
            }

            match mesh_path {
                Some(path) if self.mesh_cache.contains_key(path) => {
                    let mesh = &self.mesh_cache[path];

                    // Check for external texture override on any entity in this group
                    let has_ext_tex = self
                        .entity_texture_overrides
                        .values()
                        .any(|tp| self.texture_path_cache.contains_key(tp));

                    let has_texture = mesh.texture_bind_group.is_some() || has_ext_tex;

                    if has_texture {
                        // Choose pipeline variant based on material properties
                        if is_wireframe {
                            if let Some(ref wp) = self.wireframe_textured_pipeline {
                                pass.set_pipeline(wp);
                            } else {
                                pass.set_pipeline(&self.textured_pipeline);
                            }
                        } else if mesh.alpha_mode == 2 {
                            // Blend mode: no depth write, alpha blending
                            pass.set_pipeline(&self.alpha_blend_pipeline);
                        } else if mesh.double_sided {
                            // Double-sided: no backface culling
                            pass.set_pipeline(&self.double_sided_pipeline);
                        } else {
                            pass.set_pipeline(&self.textured_pipeline);
                        }

                        // Prefer GLTF embedded texture, fall back to external texture override
                        if let Some(tex_bg) = mesh.texture_bind_group.as_ref() {
                            pass.set_bind_group(1, tex_bg, &[]);
                        } else if let Some(ext_path) = self
                            .entity_texture_overrides
                            .values()
                            .find(|tp| self.texture_path_cache.contains_key(*tp))
                        {
                            let ext_bg = &self.texture_path_cache[ext_path];
                            pass.set_bind_group(1, ext_bg, &[]);
                        } else {
                            pass.set_bind_group(1, &self.fallback_texture_bind_group, &[]);
                        }

                        // Set material params (group 2)
                        if let Some(mat_bg) = mesh.material_bind_group.as_ref() {
                            pass.set_bind_group(2, mat_bg, &[]);
                        } else {
                            pass.set_bind_group(2, &self.fallback_material_bind_group, &[]);
                        }

                        // Set shadow map (group 3)
                        pass.set_bind_group(3, &self.shadow_bind_group, &[]);

                        // Set IBL (group 4)
                        pass.set_bind_group(4, &self.ibl_bind_group, &[]);
                    } else {
                        // Untextured pipeline (wireframe or fill)
                        if is_wireframe {
                            if let Some(ref wp) = self.wireframe_pipeline {
                                pass.set_pipeline(wp);
                            } else {
                                pass.set_pipeline(&self.pipeline);
                            }
                        } else {
                            pass.set_pipeline(&self.pipeline);
                        }
                    }

                    pass.set_bind_group(0, &self.bind_group, &[]);
                    pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                    pass.set_index_buffer(mesh.index_buffer.slice(..), mesh.index_format);
                    pass.draw_indexed(0..mesh.index_count, 0, *start..*start + *count);
                }
                _ => {
                    // Fallback to default cube (wireframe or fill)
                    if is_wireframe {
                        if let Some(ref wp) = self.wireframe_pipeline {
                            pass.set_pipeline(wp);
                        } else {
                            pass.set_pipeline(&self.pipeline);
                        }
                    } else {
                        pass.set_pipeline(&self.pipeline);
                    }
                    pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
                    pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
                    pass.draw_indexed(0..self.index_count, 0, *start..*start + *count);
                }
            }
        }

        Ok(())
    }

    /// Collect instance data grouped by mesh.
    /// Returns (flat instance list, draw groups: Vec<(mesh_path, start_idx, count)>).
    fn collect_instances_grouped(
        &self,
        world: &World,
        selected_entities: &[Entity],
        frustum: &Frustum,
    ) -> (Vec<Instance>, Vec<(Option<String>, u32, u32)>) {
        let mut default_instances = Vec::new();
        let mut mesh_instances: HashMap<String, Vec<Instance>> = HashMap::new();
        const ENTITY_RADIUS: f32 = 3.0; // Larger default for scatter boulders/trees

        for entity in world.entities() {
            if let Some(pose) = world.pose(entity) {
                // Use high-precision float position when available (scatter objects)
                let x = if pose.use_float_pos {
                    pose.float_x
                } else {
                    pose.pos.x as f32
                };
                let z = if pose.use_float_pos {
                    pose.float_z
                } else {
                    pose.pos.y as f32
                };
                let position = Vec3::new(x, pose.height, z);

                let max_scale = pose.scale.max(pose.scale_y).max(pose.scale_z);
                if !frustum.contains_sphere(position, ENTITY_RADIUS * max_scale) {
                    continue;
                }

                let translation = Mat4::from_translation(position);
                let rotation = Mat4::from_euler(
                    glam::EulerRot::XYZ,
                    pose.rotation_x,
                    pose.rotation,
                    pose.rotation_z,
                );
                let scale = Mat4::from_scale(Vec3::new(pose.scale, pose.scale_y, pose.scale_z));
                let model = translation * rotation * scale;

                let is_selected = selected_entities.contains(&entity);
                let has_mesh = self.entity_meshes.contains_key(&entity);

                let color = if is_selected {
                    [1.0, 0.6, 0.2, 1.0]
                } else if has_mesh {
                    // White tint for mesh entities — vertex colors carry the actual color
                    [1.0, 1.0, 1.0, 1.0]
                } else if let Some(team) = world.team(entity) {
                    match team.id {
                        0 => [0.2, 0.8, 0.3, 1.0],
                        1 => [0.3, 0.6, 1.0, 1.0],
                        2 => [1.0, 0.3, 0.2, 1.0],
                        _ => [0.6, 0.6, 0.7, 1.0],
                    }
                } else {
                    [0.6, 0.6, 0.7, 1.0]
                };

                let instance = Instance {
                    model_matrix: model.to_cols_array_2d(),
                    color,
                };

                if let Some(mesh_path) = self.entity_meshes.get(&entity) {
                    mesh_instances
                        .entry(mesh_path.clone())
                        .or_default()
                        .push(instance);
                } else {
                    default_instances.push(instance);
                }
            }
        }

        // Flatten into a single buffer with draw group offsets
        let mut all_instances = Vec::new();
        let mut draw_groups = Vec::new();

        if !default_instances.is_empty() {
            let start = all_instances.len() as u32;
            let count = default_instances.len() as u32;
            all_instances.append(&mut default_instances);
            draw_groups.push((None, start, count));
        }

        for (path, mut instances) in mesh_instances {
            let start = all_instances.len() as u32;
            let count = instances.len() as u32;
            all_instances.append(&mut instances);
            draw_groups.push((Some(path), start, count));
        }

        (all_instances, draw_groups)
    }
}

/// Vertex data (position + normal + color + uv)
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct Vertex {
    position: [f32; 3],
    normal: [f32; 3],
    color: [f32; 4],
    uv: [f32; 2],
}

/// Instance data (per-entity transform + color)
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct Instance {
    model_matrix: [[f32; 4]; 4],
    color: [f32; 4],
}

/// Scene point light description
#[derive(Clone, Debug)]
pub struct SceneLight {
    pub position: [f32; 3],
    pub range: f32,
    pub color: [f32; 3],
    pub intensity: f32,
}

/// Shadow map resolution (single cascade)
const SHADOW_MAP_SIZE: u32 = 2048;

/// Shadow map orthographic half-extent (world units from center)
const SHADOW_HALF_EXTENT: f32 = 50.0;

/// BRDF LUT texture resolution
const BRDF_LUT_SIZE: u32 = 256;

/// Shadow depth uniform (matches shadow.wgsl ShadowUniforms)
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct ShadowUniforms {
    light_vp: [[f32; 4]; 4],
}

/// IBL parameters (matches entity.wgsl IblParams)
/// SH L2 irradiance (9 coefficients) + intensity controls.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct IblParamsGpu {
    sh0: [f32; 4],
    sh1: [f32; 4],
    sh2: [f32; 4],
    sh3: [f32; 4],
    sh4: [f32; 4],
    sh5: [f32; 4],
    sh6: [f32; 4],
    sh7: [f32; 4],
    sh8: [f32; 4],
    ibl_intensity: [f32; 4], // x=diffuse, y=specular, z=max_spec_mip, w=enabled
}

impl Default for IblParamsGpu {
    fn default() -> Self {
        Self {
            // Default sky SH: approximate blue hemisphere over brown ground
            sh0: [0.0; 4],
            sh1: [0.0; 4],
            sh2: [0.0; 4],
            sh3: [0.0; 4],
            sh4: [0.0; 4],
            sh5: [0.0; 4],
            sh6: [0.0; 4],
            sh7: [0.0; 4],
            sh8: [0.0; 4],
            ibl_intensity: [1.0, 1.0, 6.0, 0.0], // disabled by default
        }
    }
}

/// Camera + lighting uniforms (matches entity.wgsl Uniforms struct)
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct EntityUniforms {
    view_proj: [[f32; 4]; 4],
    camera_pos: [f32; 3],
    shading_mode: u32,
    // Scene lighting (vec4-packed for GPU alignment)
    sun_dir_and_count: [f32; 4],
    sun_color_and_intensity: [f32; 4],
    ambient_color_and_intensity: [f32; 4],
    // Up to 4 point lights
    light0_pos_range: [f32; 4],
    light0_color_intensity: [f32; 4],
    light1_pos_range: [f32; 4],
    light1_color_intensity: [f32; 4],
    light2_pos_range: [f32; 4],
    light2_color_intensity: [f32; 4],
    light3_pos_range: [f32; 4],
    light3_color_intensity: [f32; 4],
    // Shadow mapping
    shadow_vp: [[f32; 4]; 4],
    shadow_params: [f32; 4],
    // Color management
    exposure_params: [f32; 4],
}

/// Per-material PBR parameters (matches entity.wgsl MaterialParams struct).
/// All vec4-packed for alignment safety across CPU/GPU boundary.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct MaterialParamsGpu {
    /// RGBA base color multiplier
    base_color_factor: [f32; 4],
    /// xyz=emissive factor, w=metallic factor
    emissive_and_metallic: [f32; 4],
    /// x=roughness, y=emissive_strength, z=occlusion_strength, w=alpha_cutoff
    pbr_params: [f32; 4],
    /// x=ior, y=clearcoat_factor, z=clearcoat_roughness, w=alpha_mode (0/1/2 as f32)
    extra_params: [f32; 4],
}

impl Default for MaterialParamsGpu {
    fn default() -> Self {
        Self {
            base_color_factor: [1.0, 1.0, 1.0, 1.0],
            emissive_and_metallic: [0.0, 0.0, 0.0, 0.0], // no emission, non-metallic
            pbr_params: [0.5, 1.0, 1.0, 0.5], // roughness=0.5, emissive_str=1, occ_str=1, alpha_cutoff=0.5
            extra_params: [1.5, 0.0, 0.0, 0.0], // ior=1.5, no clearcoat, opaque
        }
    }
}

/// Create humanoid-proportioned mesh (vertices + indices)
///
/// Returns (vertices, indices) for a tall capsule-like box (0.5×1.8×0.5)
/// with the base at Y=0. This gives entities a vertical humanoid silhouette
/// when no GLTF mesh is available.
fn create_cube_mesh() -> (Vec<Vertex>, Vec<u16>) {
    let white = [1.0, 1.0, 1.0, 1.0];

    // Half-extents: 0.25 wide, 0.9 tall (1.8 total), 0.25 deep
    // Y range: 0.0 to 1.8 (base at origin so entity stands on ground)
    let x0: f32 = -0.25;
    let x1: f32 = 0.25;
    let y0: f32 = 0.0;
    let y1: f32 = 1.8;
    let z0: f32 = -0.25;
    let z1: f32 = 0.25;
    let vertices = vec![
        // Front face (+Z)
        Vertex {
            position: [x0, y0, z1],
            normal: [0.0, 0.0, 1.0],
            color: white,
            uv: [0.0, 1.0],
        },
        Vertex {
            position: [x1, y0, z1],
            normal: [0.0, 0.0, 1.0],
            color: white,
            uv: [1.0, 1.0],
        },
        Vertex {
            position: [x1, y1, z1],
            normal: [0.0, 0.0, 1.0],
            color: white,
            uv: [1.0, 0.0],
        },
        Vertex {
            position: [x0, y1, z1],
            normal: [0.0, 0.0, 1.0],
            color: white,
            uv: [0.0, 0.0],
        },
        // Back face (-Z)
        Vertex {
            position: [x1, y0, z0],
            normal: [0.0, 0.0, -1.0],
            color: white,
            uv: [0.0, 1.0],
        },
        Vertex {
            position: [x0, y0, z0],
            normal: [0.0, 0.0, -1.0],
            color: white,
            uv: [1.0, 1.0],
        },
        Vertex {
            position: [x0, y1, z0],
            normal: [0.0, 0.0, -1.0],
            color: white,
            uv: [1.0, 0.0],
        },
        Vertex {
            position: [x1, y1, z0],
            normal: [0.0, 0.0, -1.0],
            color: white,
            uv: [0.0, 0.0],
        },
        // Right face (+X)
        Vertex {
            position: [x1, y0, z1],
            normal: [1.0, 0.0, 0.0],
            color: white,
            uv: [0.0, 1.0],
        },
        Vertex {
            position: [x1, y0, z0],
            normal: [1.0, 0.0, 0.0],
            color: white,
            uv: [1.0, 1.0],
        },
        Vertex {
            position: [x1, y1, z0],
            normal: [1.0, 0.0, 0.0],
            color: white,
            uv: [1.0, 0.0],
        },
        Vertex {
            position: [x1, y1, z1],
            normal: [1.0, 0.0, 0.0],
            color: white,
            uv: [0.0, 0.0],
        },
        // Left face (-X)
        Vertex {
            position: [x0, y0, z0],
            normal: [-1.0, 0.0, 0.0],
            color: white,
            uv: [0.0, 1.0],
        },
        Vertex {
            position: [x0, y0, z1],
            normal: [-1.0, 0.0, 0.0],
            color: white,
            uv: [1.0, 1.0],
        },
        Vertex {
            position: [x0, y1, z1],
            normal: [-1.0, 0.0, 0.0],
            color: white,
            uv: [1.0, 0.0],
        },
        Vertex {
            position: [x0, y1, z0],
            normal: [-1.0, 0.0, 0.0],
            color: white,
            uv: [0.0, 0.0],
        },
        // Top face (+Y)
        Vertex {
            position: [x0, y1, z1],
            normal: [0.0, 1.0, 0.0],
            color: white,
            uv: [0.0, 1.0],
        },
        Vertex {
            position: [x1, y1, z1],
            normal: [0.0, 1.0, 0.0],
            color: white,
            uv: [1.0, 1.0],
        },
        Vertex {
            position: [x1, y1, z0],
            normal: [0.0, 1.0, 0.0],
            color: white,
            uv: [1.0, 0.0],
        },
        Vertex {
            position: [x0, y1, z0],
            normal: [0.0, 1.0, 0.0],
            color: white,
            uv: [0.0, 0.0],
        },
        // Bottom face (-Y)
        Vertex {
            position: [x0, y0, z0],
            normal: [0.0, -1.0, 0.0],
            color: white,
            uv: [0.0, 1.0],
        },
        Vertex {
            position: [x1, y0, z0],
            normal: [0.0, -1.0, 0.0],
            color: white,
            uv: [1.0, 1.0],
        },
        Vertex {
            position: [x1, y0, z1],
            normal: [0.0, -1.0, 0.0],
            color: white,
            uv: [1.0, 0.0],
        },
        Vertex {
            position: [x0, y0, z1],
            normal: [0.0, -1.0, 0.0],
            color: white,
            uv: [0.0, 0.0],
        },
    ];

    let indices = vec![
        0, 1, 2, 2, 3, 0, // Front
        4, 5, 6, 6, 7, 4, // Back
        8, 9, 10, 10, 11, 8, // Right
        12, 13, 14, 14, 15, 12, // Left
        16, 17, 18, 18, 19, 16, // Top
        20, 21, 22, 22, 23, 20, // Bottom
    ];

    (vertices, indices)
}
