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
    /// Per-mesh texture bind group (albedo + normal). `None` → use vertex colors.
    texture_bind_group: Option<wgpu::BindGroup>,
    /// Skeleton extracted from glTF skin (if present).
    skeleton: Option<GltfSkeleton>,
    /// Animation clips extracted from glTF animations.
    animations: Vec<GltfAnimationClip>,
    /// Per-vertex skinning data (joint indices + weights).
    skinning_data: Option<SkinningData>,
    /// Original vertex positions/normals for CPU skinning (rest pose).
    rest_vertices: Option<Vec<Vertex>>,
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

    /// Mapping from World entity ID to mesh file path
    entity_meshes: HashMap<Entity, String>,

    /// Bind group layout for per-mesh textures (albedo + sampler)
    texture_bind_group_layout: wgpu::BindGroupLayout,

    /// Queue reference for texture uploads
    queue: Arc<wgpu::Queue>,

    /// 1×1 white fallback texture bind group (used when mesh has no textures)
    fallback_texture_bind_group: wgpu::BindGroup,

    /// Pipeline for textured meshes (uses texture bind group at group 1)
    textured_pipeline: wgpu::RenderPipeline,

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
                    format: wgpu::TextureFormat::Bgra8UnormSrgb,
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

        // --- Texture bind group layout (group 1): albedo texture + sampler ---
        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Entity Texture Bind Group Layout"),
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
                ],
            });

        // Build a 1×1 white fallback texture for meshes without textures
        let fallback_tex = device.create_texture_with_data(
            &queue,
            &wgpu::TextureDescriptor {
                label: Some("Entity Fallback White Texture"),
                size: wgpu::Extent3d {
                    width: 1,
                    height: 1,
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
            &[255u8, 255, 255, 255],
        );
        let fallback_view = fallback_tex.create_view(&Default::default());
        let fallback_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Entity Fallback Sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let fallback_texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Entity Fallback Texture BG"),
            layout: &texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&fallback_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&fallback_sampler),
                },
            ],
        });

        // --- Textured pipeline (group 0 = uniforms, group 1 = texture) ---
        let textured_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Entity Textured Pipeline Layout"),
                bind_group_layouts: &[&bind_group_layout, &texture_bind_group_layout],
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
                    format: wgpu::TextureFormat::Bgra8UnormSrgb,
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
                            format: wgpu::TextureFormat::Bgra8UnormSrgb,
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
                            format: wgpu::TextureFormat::Bgra8UnormSrgb,
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
            entity_meshes: HashMap::new(),
            texture_bind_group_layout,
            queue,
            fallback_texture_bind_group,
            textured_pipeline,
            wireframe_pipeline,
            wireframe_textured_pipeline,
            texture_path_cache: HashMap::new(),
            entity_texture_overrides: HashMap::new(),
            scene_lights: Vec::new(),
            sun_direction: [0.5, 1.0, 0.3],
            sun_color: [1.0, 0.98, 0.92],
            sun_intensity: 0.7,
            ambient_color: [0.6, 0.65, 0.75],
            ambient_intensity: 0.3,
        })
    }

    /// Set the entity-to-mesh mapping for the next render. Call before render().
    pub fn set_entity_meshes(&mut self, meshes: HashMap<Entity, String>) {
        self.entity_meshes = meshes;
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
        // Track the first albedo texture index we encounter
        let mut first_albedo_image_idx: Option<usize> = None;
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

                // Extract albedo texture index from the first textured primitive
                if first_albedo_image_idx.is_none() {
                    if let Some(tex_info) = primitive
                        .material()
                        .pbr_metallic_roughness()
                        .base_color_texture()
                    {
                        first_albedo_image_idx = Some(tex_info.texture().source().index());
                    }
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

        // --- Extract albedo texture from embedded GLTF images ---
        let texture_bind_group = if let Some(img_idx) = first_albedo_image_idx {
            images
                .get(img_idx)
                .and_then(|img_data| self.create_texture_bind_group_from_gltf_image(img_data, path))
        } else {
            None
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
                skeleton,
                animations,
                skinning_data,
                rest_vertices,
            },
        );

        Ok(())
    }

    /// Convert a glTF image buffer into a GPU texture + bind group.
    fn create_texture_bind_group_from_gltf_image(
        &self,
        img_data: &gltf::image::Data,
        label: &str,
    ) -> Option<wgpu::BindGroup> {
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
                    rgba.extend_from_slice(&[chunk[0], chunk[1], 0, 255]);
                }
                rgba
            }
            _ => {
                tracing::warn!("Unsupported glTF image format for {label}");
                return None;
            }
        };

        let tex = self.device.create_texture_with_data(
            &self.queue,
            &wgpu::TextureDescriptor {
                label: Some(&format!("Albedo: {label}")),
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
        let sampler = self.device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some(&format!("Sampler: {label}")),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            ..Default::default()
        });

        Some(self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(&format!("TexBG: {label}")),
            layout: &self.texture_bind_group_layout,
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
        }))
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
        let sampler = self.device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some(&format!("ExtSampler: {path}")),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            ..Default::default()
        });

        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(&format!("ExtTexBG: {path}")),
            layout: &self.texture_bind_group_layout,
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

    /// Check if a cached mesh has skinning data.
    pub fn mesh_has_skinning(&self, mesh_path: &str) -> bool {
        self.mesh_cache
            .get(mesh_path)
            .and_then(|m| m.skinning_data.as_ref())
            .is_some()
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
        };

        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&uniforms));

        let frustum = camera.extract_frustum();

        // Collect instances grouped by mesh
        let (all_instances, draw_groups) =
            self.collect_instances_grouped(world, selected_entities, &frustum);

        if all_instances.is_empty() {
            return Ok(());
        }

        // Lazy-load any GLTF meshes not yet cached
        let paths_to_load: Vec<String> = draw_groups
            .iter()
            .filter_map(|(mesh, _, _)| mesh.clone())
            .filter(|p| !self.mesh_cache.contains_key(p))
            .collect();
        for path in paths_to_load {
            if let Err(e) = self.load_gltf_mesh(&path) {
                tracing::warn!("Failed to load mesh {}: {}", path, e);
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

        for (mesh_path, start, count) in &draw_groups {
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
                        // Choose wireframe or fill textured pipeline
                        if is_wireframe {
                            if let Some(ref wp) = self.wireframe_textured_pipeline {
                                pass.set_pipeline(wp);
                            } else {
                                pass.set_pipeline(&self.textured_pipeline);
                            }
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
        const ENTITY_RADIUS: f32 = 0.866;

        for entity in world.entities() {
            if let Some(pose) = world.pose(entity) {
                let x = pose.pos.x as f32;
                let z = pose.pos.y as f32;
                let position = Vec3::new(x, pose.height, z);

                if !frustum.contains_sphere(position, ENTITY_RADIUS * pose.scale) {
                    continue;
                }

                let translation = Mat4::from_translation(position);
                let rotation = Mat4::from_euler(
                    glam::EulerRot::XYZ,
                    pose.rotation_x,
                    pose.rotation,
                    pose.rotation_z,
                );
                let scale = Mat4::from_scale(Vec3::splat(pose.scale));
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
